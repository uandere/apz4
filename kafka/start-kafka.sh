#!/bin/bash

echo "Waiting for Zookeeper to be ready..."
while ! nc -z zookeeper 2181; do
  sleep 1
done
echo "Zookeeper is ready."

/etc/confluent/docker/run &

echo "Waiting for Kafka to start up..."
sleep 10

kafka-topics --create --topic messages --partitions 1 --replication-factor 1 --if-not-exists --bootstrap-server kafka:9092

wait
