#!/usr/bin/env bash

# buf is required see:https://docs.buf.build/installation
#
# protoc plugins required
# cargo install protoc-gen-prost-crate --version 0.1.5
# cargo install protoc-gen-tonic --version 0.1.0
# cargo install protoc-gen-prost-crate --version 0.1.5
#
# TODO:(hexfusion) add version checks
# https://github.com/neoeinstein/protoc-gen-prost/issues/5

BUF_VERSION='1.4.0'

if ! [[ "$0" =~ scripts/protobuf_codegen.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

if [[ $(buf --version | cut -f2 -d' ') != "${BUF_VERSION}" ]]; then
  echo "could not find buf ${BUF_VERSION}, is it installed + in PATH?"
  exit 255
fi

if ! [ -x "$(command -v protoc-gen-prost)" ]; then
  echo "could not find protoc-gen-prost, is it installed + in PATH?"
  exit 255
fi

if ! [ -x "$(command -v protoc-gen-tonic)" ]; then
  echo "could not find protoc-gen-tonic, is it installed + in PATH?"
  exit 255
fi

if ! [ -x "$(command -v protoc-gen-tonic)" ]; then
  echo "could not find protoc-gen-tonic, is it installed + in PATH?"
  exit 255
fi

TARGET=$PWD/proto
if [ -n "$1" ]; then 
  TARGET="$1"
fi

# move to proto dir
cd $TARGET

## TODO(hexfusion): Remove from avalanchego
# remove duplicate promethus proto
rm -f protos/avalanchego/proto/io/prometheus/client/client.proto

echo "Re-generating protobuf..."

buf generate

if [[ $? -ne 0 ]];  then
    echo "ERROR: protobuf generation failed"
    exit 1
fi

# reset submodule
git submodule foreach --recursive git reset --hard 1> /dev/null

