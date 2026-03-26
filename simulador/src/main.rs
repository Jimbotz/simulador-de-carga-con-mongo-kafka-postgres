use csv::ReaderBuilder;
use futures::stream::StreamExt;
use hdrhistogram::Histogram;
use mongodb::{
    bson::{doc, oid::ObjectId},
    options::{AggregateOptions, ClientOptions, IndexOptions},
    Client, Collection, IndexModel,
};
use rand::{
    distributions::WeightedIndex,
    prelude::Distribution,
    rngs::StdRng,
    seq::SliceRandom,
    Rng, SeedableRng,
};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fs::File,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};
use tokio::{sync::mpsc, time::Duration};

// ─────────────────────────────────────────────
//  CONFIGURACIÓN GLOBAL
// ─────────────────────────────────────────────
// La URI se lee desde la variable de entorno MONGO_URI inyectada por docker-compose.
// Si no existe, se usa el valor local para desarrollo directo (cargo run).
fn mongo_uri() -> String {
    //std::env::var("MONGO_URI").unwrap_or_else(|_| {
    //    "mongodb://admin:testing@localhost:46100/?authSource=admin&directConnection=true" // para 3 replica set mongodb://admin:testing@mongodb1:27017,mongodb2:27017,mongodb3:27017/?authSource=admin&replicaSet=rs0
    //        .to_string()
    //})
    std::env::var("MONGO_URI").expect("MONGO_URI environment variable not set")
}
const NUM_WORKERS: usize = 4;
const REGISTROS_A_CARGAR: usize = 2_000_000;
const IDS_POR_WORKER: usize = 50_000;
const MAX_IDS_LOCALES: usize = 100_000; // techo para evitar memory leak
const DURACION_SIMULACION_SEGS: u64 = 300; // 5 minutos exactos
const REGISTROS_CALENTAMIENTO: usize = 5_000; // inserts iniciales si BD vacía

/// Pesos: READ 70% | CREATE 20% | UPDATE 8% | DELETE 2%
const PESOS: [u32; 4] = [70, 20, 8, 2];

// ─────────────────────────────────────────────
//  STRUCTS
// ─────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Usuario {
    nombre: String,
    apellido_paterno: String,
    apellido_materno: String,
    curp: String,
    rfc: String,
    email: String,
    telefono: String,
    edad: i32,
    estatus: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<mongodb::bson::DateTime>//Option<i64>,
}

/// Métricas atómicas compartidas entre workers y monitor
#[derive(Default)]
struct Metricas {
    reads: AtomicU64,
    inserts: AtomicU64,
    updates: AtomicU64,
    deletes: AtomicU64,
    errores_duplicado: AtomicU64,
    errores_otros: AtomicU64,
}

// ─────────────────────────────────────────────
//  MAIN
// ─────────────────────────────────────────────
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 Iniciando Simulador de Carga en Rust...");
    println!("⏱  Duración máxima: {} segundos (5 minutos)", DURACION_SIMULACION_SEGS);

    // Conexión a MongoDB
    let uri = mongo_uri();
    println!("🔌 Conectando a: {}", uri);
    let client_options = ClientOptions::parse(&uri).await?;
    let client = Client::with_options(client_options)?;
    let db = client.database("SyntheticDB");
    let col_docs: Collection<mongodb::bson::Document> = db.collection("Users");

    preparar_indices(&col_docs).await?;

    // Cargar CSV en RAM una sola vez
    let datos = Arc::new(cargar_dataset().await?);

    // ── Fase de calentamiento ──────────────────────────────────────────────
    // Si la BD está vacía, insertamos un lote base para que los workers
    // tengan IDs desde el primer ciclo y no desperdicien el 78% de las ops.
    let conteo_actual = col_docs.count_documents(None, None).await.unwrap_or(0);
    if conteo_actual == 0 {
        println!(
            "⚠️  Base de datos vacía detectada. Ejecutando fase de calentamiento ({} inserts)...",
            REGISTROS_CALENTAMIENTO
        );
        fase_calentamiento(&col_docs, &datos, REGISTROS_CALENTAMIENTO).await;
        println!("✅ Calentamiento completo.");
    } else {
        println!(
            "ℹ️  BD con ~{} registros. Saltando calentamiento.",
            conteo_actual
        );
    }

    // Señal de parada compartida
    let running = Arc::new(AtomicBool::new(true));

    // Métricas compartidas
    let metricas = Arc::new(Metricas::default());

    // Canal de latencias
    let (tx_latencias, rx_latencias) = mpsc::unbounded_channel::<u64>();

    // Tarea de monitoreo (imprime cada segundo)
    {
        let running_mon = Arc::clone(&running);
        let metricas_mon = Arc::clone(&metricas);
        tokio::spawn(monitor_metricas(rx_latencias, running_mon, metricas_mon));
    }

    // Temporizador de parada
    {
        let running_timer = Arc::clone(&running);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(DURACION_SIMULACION_SEGS)).await;
            println!("\n🛑 Tiempo límite alcanzado. Enviando señal de parada...");
            running_timer.store(false, Ordering::SeqCst);
        });
    }

    // Lanzar workers
    let mut handles = vec![];
    for worker_id in 0..NUM_WORKERS {
        let col_clone = col_docs.clone();
        let datos_clone = Arc::clone(&datos);
        let tx_clone = tx_latencias.clone();
        let running_clone = Arc::clone(&running);
        let metricas_clone = Arc::clone(&metricas);

        handles.push(tokio::spawn(async move {
            trabajador(
                worker_id,
                col_clone,
                datos_clone,
                tx_clone,
                running_clone,
                metricas_clone,
            )
            .await;
        }));
    }

    // Esperar a que todos los workers terminen
    for h in handles {
        let _ = h.await;
    }

    // Resumen final
    println!("\n══════════════════════════════════════════");
    println!("📊 RESUMEN FINAL");
    println!("══════════════════════════════════════════");
    println!(
        "  Reads   : {}",
        metricas.reads.load(Ordering::Relaxed)
    );
    println!(
        "  Inserts : {}",
        metricas.inserts.load(Ordering::Relaxed)
    );
    println!(
        "  Updates : {}",
        metricas.updates.load(Ordering::Relaxed)
    );
    println!(
        "  Deletes : {}",
        metricas.deletes.load(Ordering::Relaxed)
    );
    println!(
        "  Errores duplicado : {}",
        metricas.errores_duplicado.load(Ordering::Relaxed)
    );
    println!(
        "  Errores otros     : {}",
        metricas.errores_otros.load(Ordering::Relaxed)
    );
    println!("══════════════════════════════════════════");
    println!("✅ Simulación finalizada.");

    Ok(())
}

// ─────────────────────────────────────────────
//  FASE DE CALENTAMIENTO
// ─────────────────────────────────────────────
async fn fase_calentamiento(
    col: &Collection<mongodb::bson::Document>,
    datos: &[Usuario],
    cantidad: usize,
) {
    let mut rng = StdRng::from_entropy();
    let muestra: Vec<&Usuario> = datos.choose_multiple(&mut rng, cantidad).collect();

    for usuario in muestra {
        let doc = construir_doc_usuario(usuario, "activo");
        // Ignoramos errores de duplicado en calentamiento
        let _ = col.insert_one(doc, None).await;
    }
}

// ─────────────────────────────────────────────
//  WORKER
// ─────────────────────────────────────────────
async fn trabajador(
    worker_id: usize,
    col: Collection<mongodb::bson::Document>,
    datos: Arc<Vec<Usuario>>,
    tx_latencias: mpsc::UnboundedSender<u64>,
    running: Arc<AtomicBool>,
    metricas: Arc<Metricas>,
) {
    let mut rng = StdRng::from_entropy();
    let dist = WeightedIndex::new(&PESOS).unwrap();

    // Carga IDs usando $sample para distribución aleatoria real sobre toda la BD
    let mut ids_locales = cargar_ids_existentes_sample(&col, IDS_POR_WORKER).await;
    println!(
        "✅ Worker {} listo con {} IDs locales",
        worker_id,
        ids_locales.len()
    );

    // ── Loop principal ─────────────────────────────────────────────────────
    while running.load(Ordering::Relaxed) {
        let accion_idx = dist.sample(&mut rng);
        let start = Instant::now();

        match accion_idx {
            // ── READ (70%) ────────────────────────────────────────────────
            0 => {
                if !ids_locales.is_empty() {
                    let id = ids_locales.choose(&mut rng).unwrap();
                    let _ = col.find_one(doc! { "_id": id }, None).await;
                    metricas.reads.fetch_add(1, Ordering::Relaxed);
                }
            }

            // ── CREATE (20%) ──────────────────────────────────────────────
            1 => {
                if let Some(usuario) = datos.choose(&mut rng) {
                    // Variamos el email para reducir colisiones en ejecuciones largas
                    //let sufijo: u32 = rng.gen_range(0..9_999_999);
                    //let email_unico = format!(
                    //    "{}.{}@synth.local",
                    //    &usuario.email.split('@').next().unwrap_or("u"),
                    //    sufijo
                    //);
                    let mut doc = construir_doc_usuario(usuario, "activo");
                    //doc.insert("email", email_unico);

                    match col.insert_one(doc, None).await {
                        Ok(res) => {
                            metricas.inserts.fetch_add(1, Ordering::Relaxed);
                            if let Some(id) = res.inserted_id.as_object_id() {
                                ids_locales.push(id);
                                // Sliding window: si excede el techo, descartamos los más viejos
                                if ids_locales.len() > MAX_IDS_LOCALES {
                                    ids_locales.drain(0..10_000);
                                }
                            }
                        }
                        Err(e) => {
                            if es_error_duplicado(&e) {
                                metricas.errores_duplicado.fetch_add(1, Ordering::Relaxed);
                            } else {
                                metricas.errores_otros.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                }
            }

            // ── UPDATE (8%) ───────────────────────────────────────────────
            2 => {
                if !ids_locales.is_empty() {
                    let id = *ids_locales.choose(&mut rng).unwrap();
                    let nueva_edad: i32 = rng.gen_range(18..=90);

                    match col
                        .update_one(
                            doc! { "_id": id, "estatus": { "$ne": "eliminado" } },
                            doc! { "$set": { "edad": nueva_edad, "estatus": "actualizado" } },
                            None,
                        )
                        .await
                    {
                        Ok(res) => {
                            metricas.updates.fetch_add(1, Ordering::Relaxed);
                            // Si no hubo match, el registro fue eliminado → lo quitamos
                            if res.matched_count == 0 {
                                ids_locales.retain(|x| *x != id);
                            }
                        }
                        Err(_) => {
                            metricas.errores_otros.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }

            // ── DELETE LÓGICO (2%) ────────────────────────────────────────
            3 => {
                // Guardamos al menos 1 000 IDs para no quedarnos sin lecturas
                if ids_locales.len() > 1_000 {
                    let idx = rng.gen_range(0..ids_locales.len());
                    let id_del = ids_locales[idx];
                    let ts = mongodb::bson::DateTime::now();   //ahora_unix();

                    match col
                        .update_one(
                            doc! { "_id": id_del, "estatus": { "$ne": "eliminado" } },
                            doc! { "$set": { "estatus": "eliminado", "deleted_at": ts } },
                            None,
                        )
                        .await
                    {
                        Ok(res) => {
                            metricas.deletes.fetch_add(1, Ordering::Relaxed);
                            if res.matched_count > 0 {
                                ids_locales.remove(idx);
                            }
                        }
                        Err(_) => {
                            metricas.errores_otros.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                }
            }

            _ => {}
        }

        let latencia_us = start.elapsed().as_micros() as u64;
        let _ = tx_latencias.send(latencia_us);
    }

    println!("👷 Worker {} detenido.", worker_id);
}

// ─────────────────────────────────────────────
//  MONITOR DE MÉTRICAS
// ─────────────────────────────────────────────
async fn monitor_metricas(
    mut rx_latencias: mpsc::UnboundedReceiver<u64>,
    running: Arc<AtomicBool>,
    metricas: Arc<Metricas>,
) {
    let mut hist = Histogram::<u64>::new(3).unwrap();
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut segundos = 0u64;

    loop {
        interval.tick().await;
        segundos += 1;

        // Drenar todas las latencias pendientes en el canal
        while let Ok(lat) = rx_latencias.try_recv() {
            let _ = hist.record(lat);
        }

        let ops = hist.len();
        if ops > 0 {
            let avg = hist.mean() / 1_000.0;
            let p95 = hist.value_at_percentile(95.0) as f64 / 1_000.0;
            let p99 = hist.value_at_percentile(99.0) as f64 / 1_000.0;

            println!(
                "[{:>3}s] 🔥 {:>6} ops/s | Avg {:>6.2}ms | P95 {:>6.2}ms | P99 {:>6.2}ms \
                 | R:{} W:{} U:{} D:{} Err:{}",
                segundos,
                ops,
                avg,
                p95,
                p99,
                metricas.reads.load(Ordering::Relaxed),
                metricas.inserts.load(Ordering::Relaxed),
                metricas.updates.load(Ordering::Relaxed),
                metricas.deletes.load(Ordering::Relaxed),
                metricas.errores_duplicado.load(Ordering::Relaxed)
                    + metricas.errores_otros.load(Ordering::Relaxed),
            );
        } else {
            println!("[{:>3}s] ⏳ Esperando tráfico...", segundos);
        }

        hist.clear();

        if !running.load(Ordering::Relaxed) {
            break;
        }
    }
}

// ─────────────────────────────────────────────
//  CARGA DE IDs con $sample (aleatoriedad real)
// ─────────────────────────────────────────────
/// Usa el pipeline de agregación $sample para obtener IDs
/// distribuidos uniformemente sobre TODA la colección,
/// sin importar si tiene 1 000 o 2 000 000 registros.
async fn cargar_ids_existentes_sample(
    col: &Collection<mongodb::bson::Document>,
    limite: usize,
) -> Vec<ObjectId> {
    let pipeline = vec![
        doc! { "$match": { "estatus": { "$ne": "eliminado" } } },
        doc! { "$sample": { "size": limite as i64 } },
        doc! { "$project": { "_id": 1 } },
    ];

    let mut ids = Vec::with_capacity(limite);
    let opts = AggregateOptions::builder().build();

    if let Ok(mut cursor) = col.aggregate(pipeline, opts).await {
        while let Some(Ok(doc)) = cursor.next().await {
            if let Ok(id) = doc.get_object_id("_id") {
                ids.push(id);
            }
        }
    }

    // Si la colección no tenía nada (BD vacía tras calentamiento fallido),
    // devolvemos vacío y el worker operará solo en modo INSERT hasta tener IDs.
    ids
}

// ─────────────────────────────────────────────
//  CARGA DEL DATASET CSV
// ─────────────────────────────────────────────
async fn cargar_dataset() -> Result<Vec<Usuario>, Box<dyn Error>> {
    println!("📂 Cargando registros del CSV en RAM...");
    
    let csv_path = std::env::var("CSV_PATH").unwrap_or_else(|_| "./csv_data/usuarios_sinteticos.csv".to_string());
    let file = File::open(&csv_path)?;
    
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
    let mut datos = Vec::with_capacity(REGISTROS_A_CARGAR);

    for result in rdr.records().take(REGISTROS_A_CARGAR) {
        let record = result?;
        datos.push(Usuario {
            nombre: record[0].to_string(),
            apellido_paterno: record[1].to_string(),
            apellido_materno: record[2].to_string(),
            curp: record[3].to_string(),
            rfc: record[4].to_string(),
            email: record[5].to_string(),
            telefono: record[6].to_string(),
            edad: record[7].parse().unwrap_or(30),
            estatus: "activo".to_string(),
            deleted_at: None,
        });
    }

    println!("✅ Registros cargados en RAM: {}", datos.len());
    Ok(datos)
}

// ─────────────────────────────────────────────
//  PREPARACIÓN DE ÍNDICES
// ─────────────────────────────────────────────
async fn preparar_indices(
    col: &Collection<mongodb::bson::Document>,
) -> Result<(), Box<dyn Error>> {
    // Ignorar error si la colección no existe aún (primera ejecución)
    let _ = col.drop_indexes(None).await;

    let unique_index = IndexModel::builder()
        .keys(doc! { "curp": 1, "rfc": 1, "email": 1 })
        .options(IndexOptions::builder().unique(true).build())
        .build();

    let estatus_index = IndexModel::builder()
        .keys(doc! { "estatus": 1 })
        .build();

    col.create_indexes(vec![unique_index, estatus_index], None)
        .await?;

    println!("✅ Índices listos.");
    Ok(())
}

// ─────────────────────────────────────────────
//  HELPERS
// ─────────────────────────────────────────────

/// Construye un documento BSON a partir de un Usuario
fn construir_doc_usuario(
    u: &Usuario,
    estatus: &str,
) -> mongodb::bson::Document {
    doc! {
        "nombre":            &u.nombre,
        "apellido_paterno":  &u.apellido_paterno,
        "apellido_materno":  &u.apellido_materno,
        "curp":              &u.curp,
        "rfc":               &u.rfc,
        "email":             &u.email,
        "telefono":          &u.telefono,
        "edad":              u.edad,
        "estatus":           estatus,
    }
}

/// Timestamp Unix en segundos (i64)
///fn ahora_unix() -> i64 {
///    std::time::SystemTime::now()
///        .duration_since(std::time::UNIX_EPOCH)
///        .unwrap()
///        .as_secs() as i64
///}

/// Detecta error E11000 (duplicate key) de MongoDB
fn es_error_duplicado(e: &mongodb::error::Error) -> bool {
    e.to_string().contains("E11000")
}