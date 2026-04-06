# kafka-connect/Dockerfile
FROM confluentinc/cp-kafka-connect:7.5.0

# Cambiamos a root para instalar dependencias y gestionar carpetas
USER root

# 1. Instalar conectores oficiales desde Confluent Hub
RUN confluent-hub install --no-prompt mongodb/kafka-connect-mongodb:1.11.2
RUN confluent-hub install --no-prompt confluentinc/kafka-connect-jdbc:10.7.4

# 2. Instalar el SMT personalizado (Stringify JSON)
# Descargamos el archivo comprimido .tar.gz, lo extraemos en la carpeta de plugins y limpiamos
RUN curl -fSL -o /tmp/plugin.tar.gz https://github.com/max-prosper/stringify-json-smt/releases/download/0.0.4/stringify-json-smt-0.0.4.tar.gz \
    && tar -xzf /tmp/plugin.tar.gz -C /usr/share/confluent-hub-components/ \
    && rm -f /tmp/plugin.tar.gz

# Regresamos al usuario por defecto de Confluent por seguridad
USER appuser