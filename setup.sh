#!/bin/bash
# setup.sh — ejecutar UNA VEZ después de clonar el repo
# Genera el keyfile de MongoDB si no existe
set -e

KEY_PATH="security/mongodb.key"

if [ -f "$KEY_PATH" ] && [ -s "$KEY_PATH" ]; then
    echo "✅ $KEY_PATH ya existe — sin cambios."
else
    echo "🔑 Generando security/mongodb.key ..."
    mkdir -p security
    openssl rand -base64 756 > "$KEY_PATH"
    chmod 400 "$KEY_PATH"
    echo "✅ $KEY_PATH generado."
fi

echo ""
echo "Listo. Ahora puedes ejecutar:"
echo "  docker compose up --build"
