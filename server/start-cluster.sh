#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

IMAGE="idempotency-server:latest"

usage() {
  cat <<EOF
Usage: $0 <command>

Commands:
  build     Build the Docker image
  up        Build (if needed) and start the 3-node cluster
  down      Stop and remove containers (data volumes are preserved)
  destroy   Stop containers AND delete all data volumes
  logs      Tail logs from all nodes
  status    Show running containers
EOF
}

build() {
  echo "==> Building $IMAGE ..."
  docker build -t "$IMAGE" .
  echo "==> Build complete."
}

up() {
  if ! docker image inspect "$IMAGE" &>/dev/null; then
    echo "==> Image not found, building first ..."
    build
  fi
  echo "==> Starting cluster ..."
  docker compose up -d
  echo ""
  echo "Cluster is up."
  echo "Connect your client to the leader. Try each port until one accepts:"
  echo "  node1 gRPC : localhost:51000"
  echo "  node2 gRPC : localhost:51001"
  echo "  node3 gRPC : localhost:51002"
  echo ""
  echo "Tail logs with: $0 logs"
}

down() {
  echo "==> Stopping cluster (volumes preserved) ..."
  docker compose down
}

destroy() {
  echo "==> WARNING: This will delete all cluster data volumes."
  read -r -p "Are you sure? [y/N] " confirm
  if [[ "${confirm,,}" != "y" ]]; then
    echo "Aborted."
    exit 0
  fi
  docker compose down -v
  echo "==> Cluster and volumes removed."
}

logs() {
  docker compose logs -f --tail=50
}

status() {
  docker compose ps
}

case "${1:-}" in
  build)   build ;;
  up)      up ;;
  down)    down ;;
  destroy) destroy ;;
  logs)    logs ;;
  status)  status ;;
  *)       usage; exit 1 ;;
esac
