sudo docker run -p 51000:51000 -p 8100:8100 -p 8200:8200 -e RPC_ADDR="0.0.0.0:51000" -e NODES="1 localhost:8100 localhost:8200" -e DATA_DIR="/app/data" -e NODE_ID="1" -e ADDR_API="localhost:8100" -e ADDR_RAFT="localhost:8200" -v "/dev/shm/data/node_1.local:/app/data" idempotency-server

sudo apt update && sudo apt install -y libclang-dev clang
