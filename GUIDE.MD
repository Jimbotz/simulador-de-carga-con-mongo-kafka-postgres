
# Guía de ejecución: Simulador de carga automatizado (CDC Pipeline)

Este proyecto levanta un entorno completo para simular cargas de trabajo y procesar datos sintéticos, estableciendo una arquitectura **Change Data Capture (CDC)** en tiempo real.

**Componentes de la arquitectura:**

  * **`chunkattack` (Python):** Generador de 2 millones de usuarios sintéticos guardados en un CSV.
  * **`mongodb`:** Clúster NoSQL configurado como Replica Set (Origen de datos / Source).
  * **`zookeeper` & `kafka_broker`:** El sistema de mensajería (Message Broker) que encola los eventos de la base de datos.
  * **`kafka_connect` & `cdc_setup`:** El worker que lee los cambios de MongoDB y el script efímero que configura automáticamente los conectores CDC.
  * **`postgres`:** Data Warehouse relacional (Destino de los datos / Sink).
  * **`simulador_carga` (Rust):** Aplicación asíncrona de alto rendimiento que inyecta operaciones CRUD concurrentes hacia MongoDB basándose en el CSV.

-----

## 🚦 Elige tu sistema operativo

Haz clic en el sistema operativo que utilizas para ir a las instrucciones específicas de configuración inicial:

  * [▶️ Ir a las instrucciones para Windows](https://www.google.com/search?q=%23-configuraci%C3%B3n-para-windows)
  * [▶️ Ir a las instrucciones para Linux / macOS](https://www.google.com/search?q=%23-configuraci%C3%B3n-para-linux--macos)

-----

### 🪟 Configuración para Windows

**Requisitos Previos:**

  * Tener instalado **Docker Desktop** (configurado con integración WSL2).
  * Tener instalado **Git Bash** (viene incluido al instalar Git para Windows). *No uses PowerShell ni CMD para ejecutar los scripts.*

**Pasos:**

1.  Abre **Git Bash** o tu terminal de **WSL**.
2.  Clona el repositorio:
    ```bash
    git clone <URL_DE_TU_REPOSITORIO>
    cd <NOMBRE_DE_LA_CARPETA>
    ```
3.  Crea un archivo llamado `.env` en la raíz del proyecto basándote en el ejemplo (o cópialo si existe un `.env.example`):
    ```env
    # ─── MongoDB ─────────────────────────────────────────────
    MONGO_INITDB_ROOT_USERNAME=admin
    MONGO_INITDB_ROOT_PASSWORD=testing
    # ─── PostgreSQL ───────────────────────────────────────────
    POSTGRES_USER=admin
    POSTGRES_PASSWORD=testing
    POSTGRES_DB=datawarehouse
    ```
4.  Genera la llave de seguridad para MongoDB usando Bash:
    ```bash
    bash setup.sh
    ```
5.  Continúa al [Paso 3: Levantar la Infraestructura](https://www.google.com/search?q=%23-paso-3-levantar-la-infraestructura).

-----

### 🐧 Configuración para Linux / macOS

**Requisitos Previos:**

  * Tener instalado Docker y Docker Compose.
  * Tener `git` y `openssl` instalados.

**Pasos:**

1.  Abre tu terminal.
2.  Clona el repositorio:
    ```bash
    git clone <URL_DE_TU_REPOSITORIO>
    cd <NOMBRE_DE_LA_CARPETA>
    ```
3.  Crea el archivo `.env` en la raíz del proyecto:
    ```env
    MONGO_INITDB_ROOT_USERNAME=admin
    MONGO_INITDB_ROOT_PASSWORD=testing
    POSTGRES_USER=admin
    POSTGRES_PASSWORD=testing
    POSTGRES_DB=datawarehouse
    ```
4.  Da permisos de ejecución y genera la llave de seguridad de MongoDB:
    ```bash
    chmod +x setup.sh setup-cdc.sh
    ./setup.sh
    ```
5.  Continúa al [Paso 3: Levantar la Infraestructura](https://www.google.com/search?q=%23-paso-3-levantar-la-infraestructura).

-----

## 🚀 Paso 3: Levantar la Infraestructura

Una vez configurado el `.env` y las llaves de seguridad, ejecuta el siguiente comando para construir y arrancar todo el ecosistema en segundo plano:

```bash
docker compose up --build -d
```

**Secuencia de arranque automático:**

1.  Python generará el CSV en la carpeta `/csv_data`.
2.  MongoDB inicializará su Replica Set. Zookeeper y PostgreSQL iniciarán en paralelo.
3.  Kafka Broker se levantará una vez que Zookeeper esté listo.
4.  Rust esperará a que el CSV y Mongo estén listos para comenzar la inyección de carga concurrente.
5.  Kafka Connect despertará y el contenedor `cdc_setup` inyectará automáticamente los JSON de configuración para conectar Mongo con Postgres.

-----

## 📊 Paso 4: Monitoreo y Verificación

Para asegurar que los datos fluyen correctamente desde el simulador hasta el Data Warehouse, sigue estos comandos:

**1. Ver las métricas en tiempo real del simulador en Rust:**

```bash
docker logs -f simulador_carga
```

*(Presiona `Ctrl + C` para salir de los logs sin detener el programa).*

**2. Verificar que el pipeline CDC está activo:**
Asegúrate de que el worker de PostgreSQL está recibiendo datos revisando su estado:

```bash
curl -s http://localhost:46112/connectors/postgres-sink-connector/status
```

*(Deberías ver `"state":"RUNNING"`).*

**3. Verificar los datos replicados en PostgreSQL (Vía Terminal):**

```bash
docker exec -it postgres_dw psql -U admin -d datawarehouse -c "SELECT * FROM raw_users LIMIT 5;"
```

**4. Conexión visual al Data Warehouse (Vía DBeaver):**
Puedes conectar tu cliente SQL local (como DBeaver) para ver los datos replicados en tiempo real con estas credenciales:

  * **Host:** `localhost` (o `127.0.0.1`)
  * **Puerto:** `46101`
  * **Base de datos:** `datawarehouse`
  * **Usuario:** `admin`
  * **Contraseña:** `testing`

-----

## 🔄 Paso extra: Reiniciar la ejecución del simulador

Dado que el contenedor del simulador en Rust está configurado para detenerse automáticamente al terminar su trabajo o al alcanzar su límite de tiempo (5 minutos), se quedará en estado de "espera" (`exit 0`).

Si necesitas volver a disparar el ataque de carga sin tener que apagar y reconstruir toda la infraestructura, tienes dos opciones según lo que busques probar:

### Opción A: Continuar inyectando datos (Acumulativo)

Si solo quieres que el simulador despierte y siga operando sobre los datos que ya existen en MongoDB (ideal para estresar la base de datos con más volumen o probar el manejo de errores por duplicidad):

```bash
docker compose restart simulador
```

*(El contenedor leerá el CSV nuevamente y comenzará a enviar operaciones al instante).*

### Opción B: Prueba limpia desde cero (Recomendado)

Si modificaste algo en el código o quieres medir el rendimiento desde una base de datos en blanco (para que los 2 millones de registros entren limpios):

1.  **Borra los datos actuales en MongoDB** ejecutando este comando desde tu terminal (esto elimina la colección sin tener que apagar el contenedor):
    ```bash
    docker exec -it mongodb_container mongosh -u admin -p testing --authenticationDatabase admin --eval "db.getSiblingDB('SyntheticDB').Users.drop()"
    ```
2.  **Reinicia el simulador** para que comience la carga desde cero:
    ```bash
    docker compose restart simulador
    ```

-----

## 🧹 Paso 5: Limpieza y Reinicio Limpio

Si necesitas detener la simulación y **borrar todos los datos** para hacer una prueba fresca desde cero (eliminando bases de datos y reseteando la memoria de Kafka), ejecuta:

```bash
docker compose down -v
```

*(El flag `-v` destruye los volúmenes de Mongo, Postgres y los internal topics de Kafka).*

Si también quieres forzar a que Python regenere el archivo CSV desde cero en tu siguiente ejecución, elimina la carpeta local de datos:

```bash
rm -rf csv_data/
```