#!/bin/bash

PROTO_DIR="./proto"
OUT_DIR="./gen"

mkdir -p $OUT_DIR

pnpm grpc_tools_node_protoc \
  --ts_out=grpc_js:$OUT_DIR \
  --js_out=import_style=commonjs,binary:$OUT_DIR \
  --grpc_out=grpc_js:$OUT_DIR \
  -I $PROTO_DIR \
  $PROTO_DIR/*.proto
