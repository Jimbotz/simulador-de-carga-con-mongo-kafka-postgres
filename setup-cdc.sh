#!/bin/sh

echo "Esperando a que Kafka Connect se inicie..."
# Loop until Kafka Connect's API responds with a 200 OK
while [ $(curl -s -o /dev/null -w %{http_code} http://kafka-connect:8083/connectors) -ne 200 ] ; do
  echo "Kafka Connect aún no está listo. Reintentando en 5 segundos..."
  sleep 5
done

echo "Kafka Connect está activo."

# ──────────────────────────────────────────────────────────────────────────
# 1. Check and Register MongoDB Source
# ──────────────────────────────────────────────────────────────────────────
echo -e "\nComprobando si existe un conector para MongoDB..."
MONGO_STATUS=$(curl -s -o /dev/null -w %{http_code} http://kafka-connect:8083/connectors/mongo-source-connector)

if [ $MONGO_STATUS -eq 200 ]; then
  echo "El conector de MongoDB ya existe. Saltando..."
else
  echo "Registrando el conector de origen de MongoDB..."
  curl -X POST -H "Content-Type: application/json" --data @/scripts/mongo-source.json http://kafka-connect:8083/connectors
  echo -e "\nConector de MongoDB registrado correctamente."
fi

# ──────────────────────────────────────────────────────────────────────────
# 2. Check and Register PostgreSQL Sink
# ──────────────────────────────────────────────────────────────────────────
echo -e "\nComprobando si existe el receptor PostgreSQL..."
PG_STATUS=$(curl -s -o /dev/null -w %{http_code} http://kafka-connect:8083/connectors/postgres-sink-connector)

if [ $PG_STATUS -eq 200 ]; then
  echo "El conector de PostgreSQL ya existe. Saltando..."
else
  echo "Registrando el receptor PostgreSQL..."
  curl -X POST -H "Content-Type: application/json" --data @/scripts/postgres-sink.json http://kafka-connect:8083/connectors
  echo -e "\nConector de PostgreSQL registrado correctamente."
fi

echo -e "\n¡Configuración de CDC completada!"