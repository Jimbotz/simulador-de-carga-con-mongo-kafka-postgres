# SS — Stack de Simulación de Carga

Stack completo de simulación de carga para MongoDB con Replica Set. Desde un `git clone` hasta métricas en vivo con un solo comando.

---

## Servicios

| Contenedor | Imagen | Puerto host | Rol |
|---|---|---|---|
| `chunkattack` | Python 3.12 | — | Genera el CSV de 2M usuarios y termina |
| `mongodb_container` | mongo:latest | 46100 | BD principal, Replica Set rs0 |
| `postgres_dw` | postgres:15 | 46101 | Data Warehouse (futuro CDC) |
| `simulador_carga` | Rust (multi-stage) | — | Genera carga sintética sobre MongoDB |

---

## Flujo de arranque automático

```
git clone <repo>
cd ss
docker compose up --build
```

El orden garantizado por `depends_on` es:

```
chunkattack ──(exit 0)──► simulador
mongodb ──(healthy)──────► simulador
```

1. **chunkattack** arranca primero. Genera `usuarios_sinteticos.csv` en el volumen `csv_data`. Si el CSV ya existe de una ejecución anterior, lo detecta y termina en segundos.
2. **mongodb** arranca en paralelo. El entrypoint genera el keyfile automáticamente si `security/mongodb.key` no existe, inicializa `rs0` y lanza el proceso principal.
3. **simulador** arranca solo cuando ambas condiciones se cumplen: `chunkattack` terminó con exit 0 **y** `mongodb` pasó el healthcheck.
4. **postgres** arranca en paralelo sin bloquear nada.

---

## Estructura del proyecto

```
ss/
├── docker-compose.yml
├── .env                          ← Credenciales (excluido de git)
├── .gitignore
├── setup.sh                      ← Genera security/mongodb.key (solo primera vez)
├── chunkattack/
│   ├── Dockerfile                ← Python 3.12 slim
│   ├── requirements.txt          ← faker-persona-mx -- ahorita aca estamos clonando el repositorio oficial, cuando no haya problemas de duplicados podremos crear el archivo de reqirements 
│   └── generar.py                ← Genera el CSV en /data (volumen csv_data)
├── scripts/
│   └── mongo-entrypoint.sh       ← Configura keyfile + inicializa rs0
├── security/
│   └── mongodb.key               ← Generado por setup.sh o por el entrypoint
└── simulador/
    ├── Dockerfile
    ├── Cargo.toml
    ├── Cargo.lock
    ├── PATCH_CSV_PATH.md         ← Instrucciones para actualizar la ruta del CSV
    └── src/
        └── main.rs
```

---


## Variables de entorno relevantes para el simulador

| Variable | Valor en contenedor | Descripción |
|---|---|---|
| `MONGO_URI` | `mongodb://admin:testing@mongodb:27017/...` | URI de MongoDB |
| `CSV_PATH` | `/csv_data/usuarios_sinteticos.csv` | Ruta del CSV dentro del contenedor |

---

## Comandos de operación

```bash
# Ver logs del generador de CSV
docker logs -f chunkattack

# Ver logs de MongoDB
docker logs -f mongodb_container

# Ver métricas del simulador en vivo
docker logs -f simulador_carga

# Detener sin borrar datos
docker compose down

# Reset total (borra volúmenes: mongo_data, pg_data, csv_data)
docker compose down -v

# Reiniciar solo el simulador
docker compose restart simulador && docker logs -f simulador_carga
```

---

## Notas sobre el volumen csv_data

- El CSV se genera una sola vez en el volumen `csv_data` y persiste entre reinicios.
- Si haces `docker compose down -v`, el CSV se borra y `chunkattack` lo regenerará en el siguiente `up`.
- La generación de 2M registros tarda entre 3 y 8 minutos dependiendo del hardware.

---

## Solución de problemas

**`chunkattack` falla con `ModuleNotFoundError`**
Corre `docker compose build chunkattack` para reconstruir la imagen con las dependencias.

**`simulador` arranca antes de que el CSV esté listo**
Imposible por diseño: `depends_on: chunkattack: condition: service_completed_successfully` garantiza que el simulador espera el exit 0 de chunkattack.

**MongoDB queda `unhealthy` por el keyfile**
El entrypoint genera el keyfile automáticamente. Si el problema persiste, revisa que `scripts/mongo-entrypoint.sh` tenga permisos de ejecución en el host (`chmod +x scripts/mongo-entrypoint.sh`).

**El simulador no encuentra el CSV**
Verifica que la variable `CSV_PATH` esté siendo leída en `main.rs` (ver `simulador/PATCH_CSV_PATH.md`).
