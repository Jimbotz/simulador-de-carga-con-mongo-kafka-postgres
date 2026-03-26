# Cambio requerido en simulador/src/main.rs
#
# Busca la línea donde se define la ruta del CSV. Suele verse así:
#
#   let csv_path = "usuarios_sinteticos.csv";
#   // o
#   let csv_path = std::env::var("CSV_PATH").unwrap_or("usuarios_sinteticos.csv".into());
#
# Reemplázala por:
#
#   let csv_path = std::env::var("CSV_PATH")
#       .unwrap_or_else(|_| "/csv_data/usuarios_sinteticos.csv".to_string());
#
# El docker-compose.yml ya pasa CSV_PATH=/csv_data/usuarios_sinteticos.csv
# como variable de entorno al contenedor simulador.
#
# Para desarrollo local sin Docker, el fallback apunta a data/usuarios_sinteticos.csv
# (ruta relativa al directorio de trabajo donde corres `cargo run`).
