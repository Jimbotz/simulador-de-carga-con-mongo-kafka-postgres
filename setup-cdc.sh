echo "Waiting for Kafka Connect to boot up..."
# Loop until Kafka Connect's API responds with a 200 OK
while [ $(curl -s -o /dev/null -w %{http_code} http://kafka-connect:8083/connectors) -ne 200 ] ; do
  echo "Kafka Connect not ready yet. Retrying in 5 seconds..."
  sleep 5
done

echo "Kafka Connect is awake! Checking for existing MongoDB connector..."
# Check if the connector is already registered
STATUS=$(curl -s -o /dev/null -w %{http_code} http://kafka-connect:8083/connectors/mongo-source-connector)

if [ $STATUS -eq 200 ]; then
  echo " Connector already exists! Nothing to do. Exiting."
else
  echo " Fresh start detected! Registering MongoDB Source Connector..."
  curl -X POST -H "Content-Type: application/json" --data @/scripts/mongo-source.json http://kafka-connect:8083/connectors
  echo -e "\n Connector registered successfully!"
fi