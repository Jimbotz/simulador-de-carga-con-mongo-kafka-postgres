#!/bin/bash
set -e

KEY_SRC="/etc/mongodb.key"
KEY_DST="/data/mongodb.key"

echo "Configurando keyfile de MongoDB..."
if [ -f "$KEY_SRC" ] && [ -s "$KEY_SRC" ]; then
    cp "$KEY_SRC" "$KEY_DST"
    echo "   Usando keyfile existente del host."
else
    echo "   Keyfile no encontrado — generando uno temporal..."
    openssl rand -base64 756 > "$KEY_DST"
fi
chown 999:999 "$KEY_DST"
chmod 400 "$KEY_DST"
echo "Keyfile listo."

# ─── Disparar la inicialización del RS en segundo plano ────────────────
(
    echo "Esperando a que MongoDB inicie y acepte conexiones autenticadas..."
    # Esperamos a que MongoDB levante y el usuario admin exista
    until mongosh -u "${MONGO_INITDB_ROOT_USERNAME:-admin}" -p "${MONGO_INITDB_ROOT_PASSWORD:-testing}" \
        --authenticationDatabase admin --quiet --eval "db.adminCommand('ping').ok" > /dev/null 2>&1; do
        sleep 3
    done

    echo "⚙️  Intentando inicializar Replica Set rs0..."
    mongosh -u "${MONGO_INITDB_ROOT_USERNAME:-admin}" -p "${MONGO_INITDB_ROOT_PASSWORD:-testing}" \
        --authenticationDatabase admin --quiet --eval "
        try {
            if (!rs.status().ok) {
                rs.initiate({ _id: 'rs0', members: [{ _id: 0, host: 'mongodb_container:27017' }] })
            }
        } catch(e) {
            rs.initiate({ _id: 'rs0', members: [{ _id: 0, host: 'mongodb_container:27017' }] })
        }
    " # aca se tiene que hacer el cambio en la inicializacion si es que se queiren los 3 replica set recomendados: se agrega { _id: 1, host: 'mongodb2:27017' }, { _id: 2, host: 'mongodb3:27017' }

    echo "Proceso de Replica Set finalizado."
) &

# ─── Ejecutar el entrypoint oficial en el hilo principal ───────────────
echo "🚀 Cediendo el control al entrypoint oficial de MongoDB..."
exec docker-entrypoint.sh "$@"