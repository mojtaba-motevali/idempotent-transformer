#!/bin/bash

# Create data directories for each node
mkdir -p data/node_1
mkdir -p data/node_2
mkdir -p data/node_3

echo "Created data directories for cluster nodes"

# Start the cluster
docker-compose up

echo "Cluster started successfully!" 