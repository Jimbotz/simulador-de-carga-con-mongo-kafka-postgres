-----

# 🚀 Guía de Ejecución: Simulador de Carga Automatizado (CDC Pipeline)

Este proyecto levanta un entorno completo para simular cargas de trabajo y procesar datos sintéticos, preparando el terreno para una arquitectura Change Data Capture (CDC).

**Componentes de la arquitectura:**

1.  **`chunkattack` (Python):** Generador de 2 millones de usuarios sintéticos guardados en un CSV.
2.  **`mongodb`:** Clúster NoSQL configurado como Replica Set (origen de datos).
3.  **`postgres`:** Data Warehouse relacional (destino futuro del CDC).
4.  **`simulador_carga` (Rust):** Aplicación asíncrona de alto rendimiento que inyecta operaciones CRUD concurrentes hacia MongoDB basándose en el CSV.

-----

## 🚦 Elige tu Sistema Operativo

Haz clic en el sistema operativo que utilizas para ir a las instrucciones específicas de configuración inicial:

  * [▶️ Ir a las instrucciones para Windows](#-configuración-para-windows)
  * [▶️ Ir a las instrucciones para Linux / macOS](#-configuración-para-linux--macos)

  * [🔄 Ejecutar de nuevo](#-paso-extra-reiniciar-la-ejecución-del-simulador)
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
3.  Crea un archivo llamado `.env` en la raíz del proyecto con las credenciales:
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
5.  Continúa al [Paso 3: Levantar la Infraestructura](#-paso-3-levantar-la-infraestructura)

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
    chmod +x setup.sh
    ./setup.sh
    ```
5.  Continúa al [Paso 3: Levantar la Infraestructura](#-paso-3-levantar-la-infraestructura)

-----

## 🚀 Paso 3: Levantar la Infraestructura

Una vez configurado el `.env` y el `setup.sh`, ejecuta el siguiente comando para construir y arrancar todo el ecosistema en segundo plano:

```bash
docker compose up --build -d
```

**Secuencia de arranque automático:**

1.  Python generará el CSV en la carpeta `/csv_data`.
2.  MongoDB inicializará su Replica Set y creará a los usuarios.
3.  PostgreSQL se levantará en modo Data Warehouse.
4.  Rust esperará a que el CSV y Mongo estén listos para comenzar la inyección de carga concurrente.

-----

## 📊 Paso 4: Monitoreo y Verificación

**1. Ver el progreso de Python (Generación del CSV):**

```bash
docker logs -f chunkattack
```

**2. Ver las métricas en tiempo real del simulador en Rust:**

```bash
docker logs -f simulador_carga
```

*(Presiona `Ctrl + C` para salir de los logs sin detener el programa).*

**3. Verificar los datos insertados en MongoDB:**
Abre la consola interactiva ejecutando:

```bash
docker exec -it mongodb_container mongosh -u admin -p testing --authenticationDatabase admin
```

Dentro de la consola, ejecuta:

```javascript
use SyntheticDB;
db.Users.countDocuments(); // Ver total de registros
db.Users.findOne();        // Ver un registro de ejemplo
exit;                      // Salir
```

-----

## 🧹 Paso 5: Limpieza y Reinicio Limpio

Si necesitas detener la simulación y **borrar todos los datos** de las bases de datos para hacer una prueba fresca desde cero, ejecuta:

```bash
docker compose down -v
```

*(El flag `-v` destruye los volúmenes de Mongo y Postgres).*

Si también quieres forzar a que Python regenere el archivo CSV desde cero en tu siguiente ejecución, elimina la carpeta local de datos:

```bash
rm -rf csv_data/
```


Aquí tienes la nueva sección redactada en Markdown. Te sugiero insertarla justo después del **Paso 4 (Monitoreo y Verificación)** y antes del **Paso 5 (Limpieza)** en tu `README.md`.

***

## 🔄 Paso extra: Reiniciar la Ejecución del Simulador

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

1. **Borra los datos actuales en MongoDB** ejecutando este comando desde tu terminal (esto elimina la colección sin tener que apagar el contenedor):
   ```bash
   docker exec -it mongodb_container mongosh -u admin -p testing --authenticationDatabase admin --eval "db.getSiblingDB('SyntheticDB').Users.drop()"
   ```
2. **Reinicia el simulador** para que comience la carga desde cero:
   ```bash
   docker compose restart simulador
   ```

***
