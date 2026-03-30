# kafka-connect/Dockerfile
FROM confluentinc/cp-kafka-connect:7.5.0

# Install MongoDB Source/Sink Connector
RUN confluent-hub install --no-prompt mongodb/kafka-connect-mongodb:1.11.2

# Install JDBC Sink Connector (for PostgreSQL)
RUN confluent-hub install --no-prompt confluentinc/kafka-connect-jdbc:10.7.4