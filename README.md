# SS — Stack de Simulación de Carga y CDC (Change Data Capture)

Stack completo de simulación de carga para MongoDB con Replica Set y un pipeline de Change Data Capture (CDC) automatizado hacia PostgreSQL usando Kafka. Desde un `git clone` hasta métricas y replicación en vivo con un solo comando.

---

## Servicios

| Contenedor | Imagen | Puerto host | Rol |
|---|---|---|---|
| `chunkattack` | Python 3.12 | — | Genera el CSV de 2M usuarios y termina |
| `mongodb_container` | mongo:latest | 46100 | BD principal, Replica Set rs0 (Source) |
| `postgres_dw` | postgres:15 | 46101 | Data Warehouse, tabla `raw_users` (Sink) |
| `simulador_carga` | Rust (multi-stage) | — | Genera carga sintética sobre MongoDB |
| `zookeeper` | cp-zookeeper:7.5.0 | 46110 | Coordinador de cluster para Kafka |
| `kafka_broker` | cp-kafka:7.5.0 | 46111 | Message Broker (Topic: `SyntheticDB.Users`) |
| `kafka_connect` | Custom (cp-kafka-connect) | 46112 | Worker que ejecuta los conectores Mongo/Postgres |
| `cdc_setup` | curlimages/curl | — | Script efímero que auto-registra los conectores vía API |

---

## Flujo de arranque automático

```text
git clone <repo>
cp .env.example .env   # <- ¡Paso crucial antes de iniciar!
docker compose up -d --build
```

El orden garantizado por dependencias (`depends_on`) es:

1. **chunkattack** arranca primero. Genera `usuarios_sinteticos.csv` y termina.
2. **mongodb**, **postgres** y **zookeeper** arrancan en paralelo. El entrypoint de Mongo genera el keyfile e inicializa `rs0`.
3. **kafka** arranca una vez que Zookeeper está listo.
4. **simulador** arranca cuando `chunkattack` termina (exit 0) **y** `mongodb` pasa su healthcheck.
5. **kafka-connect** arranca esperando a Kafka y MongoDB.
6. **cdc_setup** espera a que la API de `kafka-connect` responda (HTTP 200) y envía automáticamente los archivos `mongo-source.json` y `postgres-sink.json` para iniciar la replicación en tiempo real.

---

## Estructura del proyecto

```text
ss/
├── docker-compose.yml
├── .env                          ← Credenciales (excluido de git)
├── .env.example                  ← Plantilla de credenciales para clones
├── .gitattributes                ← Fuerza finales de línea LF en scripts (.sh)
├── .gitignore
├── setup-cdc.sh                  ← Script de auto-registro para Kafka Connect
├── mongo-source.json             ← Configuración del conector origen (MongoDB)
├── postgres-sink.json            ← Configuración del conector destino (PostgreSQL)
├── kafka-connect/
│   └── Dockerfile                ← Imagen custom con plugins de Mongo y Postgres
├── chunkattack/
│   ├── Dockerfile
│   ├── requirements.txt
│   └── generar.py
├── scripts/
│   └── mongo-entrypoint.sh
├── security/
│   └── mongodb.key
└── simulador/
    ├── Dockerfile
    ├── Cargo.toml
    └── src/main.rs
```

---

## Variables de entorno relevantes para el simulador

| Variable | Valor en contenedor | Descripción |
|---|---|---|
| `MONGO_URI` | `mongodb://admin:testing@mongodb:27017/...` | URI de MongoDB |
| `CSV_PATH` | `/csv_data/usuarios_sinteticos.csv` | Ruta del CSV dentro del contenedor |

---

## Comandos de operación y monitoreo

```bash
# Ver métricas del simulador en vivo
docker logs -f simulador_carga

# Verificar que los conectores CDC se registraron correctamente
docker logs -f cdc_setup

# Ver el estado de salud del worker de PostgreSQL
curl -s http://localhost:46112/connectors/postgres-sink-connector/status

# Escuchar el flujo de datos en vivo desde Kafka (Topic)
docker exec -it kafka_broker kafka-console-consumer \
  --bootstrap-server localhost:29092 \
  --topic SyntheticDB.Users \
  --from-beginning 
# el --from-beginning solo aplica si quieren ver todos los registros, si se quita solo muestra desde ese momento en adelante
# Reiniciar todo desde cero (borra BDs y la memoria de Kafka)
docker compose down -v
docker compose up -d --build
```

---

## Conexión al Data Warehouse (DBeaver / DataGrip)

Para ver los datos replicados en tiempo real desde MongoDB, conéctate a la base de datos PostgreSQL desde tu máquina host utilizando los siguientes parámetros:

* **Host:** `localhost` (o `127.0.0.1`)
* **Puerto:** `46101`
* **Base de datos:** `datawarehouse`
* **Usuario:** `admin` (o el definido en tu `.env`)
* **Contraseña:** `testing` (o la definida en tu `.env`)

> **Nota CDC:** La tabla destino (`raw_users`) se crea **dinámicamente** mediante "Lazy Creation". La tabla no existirá en PostgreSQL hasta que el simulador inserte el *primer* registro en MongoDB y este viaje a través de Kafka. Las nuevas columnas añadidas en Mongo se reflejarán automáticamente en Postgres gracias a `auto.evolve`.

---

## Solución de problemas

**PostgreSQL rechaza la conexión o la tabla no existe**
Asegúrate de que el simulador esté corriendo (`docker compose restart simulador`). Kafka Connect no crea la tabla en Postgres hasta que recibe el primer evento con la estructura (Schema) de los datos.

**`cdc_setup` falla al leer los JSON o scripts**
Si desarrollas en Windows, asegúrate de que el archivo `setup-cdc.sh` tenga finales de línea `LF` (Linux) y no `CRLF`. El archivo `.gitattributes` en el repositorio previene esto al clonar.

**Los conectores marcan estado "FAILED"**
Si realizas cambios manuales en la estructura de los JSON (como habilitar/deshabilitar esquemas), la memoria interna de Kafka quedará corrupta con mensajes viejos incompatibles. La solución más limpia es un reset total: `docker compose down -v && docker compose up -d --build`.


## Transformaciones en vuelo (SMTs) y manejo de esquemas

MongoDB es una base de datos documental flexible (permite arreglos y objetos anidados complejos), mientras que PostgreSQL es estrictamente relacional (espera filas y columnas planas). Para evitar que el conector de Postgres colapse al intentar mapear tipos de datos incompatibles (como listas o matrices), el archivo `postgres-sink.json` implementa un pipeline de **Single Message Transforms (SMTs)** en dos fases:

1. **Fase 1: Blindaje (Stringify Custom):** Kafka Connect utiliza un plugin personalizado (`stringify-json-smt`) instalado en el Dockerfile. Este intercepta campos inherentemente peligrosos para SQL (arreglos de tipos mixtos, arreglos de objetos, o diccionarios con llaves dinámicas UUID) y los convierte en una cadena de texto (JSON String) segura. PostgreSQL recibe esto y lo guarda en una columna tipo `TEXT`, delegando su posterior normalización y extracción a la capa analítica (ej. usando `::jsonb` con **dbt**).
2. **Fase 2: Aplanamiento (Flatten):** Una vez que los campos volátiles han sido empaquetados como texto, el transformador nativo de Kafka aplasta cualquier objeto anidado seguro que haya quedado. Por ejemplo, un sub-objeto estático como `direccion: { calle: "X", cp: "Y" }` se aplanará y generará automáticamente las columnas `direccion_calle` y `direccion_cp`.

---

### ¿Cómo agregar o quitar campos de las transformaciones?

Si en el futuro modificas el simulador (`src/main.rs`) para inyectar nuevos campos en MongoDB que contengan **arreglos** (`[]`) o **estructuras altamente dinámicas/profundas**, **debes** proteger esos campos en el archivo `postgres-sink.json` para evitar que el pipeline CDC se rompa.

**Para blindar un nuevo campo (Convertirlo a Texto JSON):**
1. Abre el archivo `postgres-sink.json`.
2. Localiza la propiedad `"transforms.stringifyCustom.targetFields"`.
3. Añade el nombre exacto de la llave raíz (root key) separada por comas.

*Ejemplo:* Si agregas una nueva lista llamada `historial_navegacion` en tus documentos de Mongo, actualiza la propiedad así:
```json
"transforms.stringifyCustom.targetFields": "datos_mixtos,sesiones_activas,metadata_compleja,caso_especial,historial_navegacion"
```

**Para que un campo anidado se vuelva columnas relacionales:**
Si creaste un objeto estático (ej. `datos_fiscales: { rfc: "...", regimen: "..." }`) y quieres que Postgres cree las columnas nativas `datos_fiscales_rfc` y `datos_fiscales_regimen`, simplemente **NO lo incluyas** en el `targetFields`. El transformador `Flatten` de la Fase 2 lo procesará en automático.

> **⚠️ Restricciones del Aplanamiento (Flatten):**
> * No intentes aplanar objetos que contengan arreglos internos.
> * Evita anidar objetos a más de 3 o 4 niveles de profundidad. PostgreSQL tiene un **límite físico de 63 caracteres** para los nombres de las columnas. Si la concatenación (`padre_hijo_nieto...`) supera ese límite, Postgres truncará el nombre y el conector podría fallar.

**Aplicar los cambios en vivo:**
Si modificas el `postgres-sink.json` para agregar o quitar campos, debes volver a registrar el conector forzando la ejecución del script efímero:
```bash
docker compose restart cdc_setup
```
*(Nota: Si los cambios de esquema generan conflictos de compatibilidad con datos previamente insertados, realiza un reinicio limpio de la infraestructura con `docker compose down -v`).*
