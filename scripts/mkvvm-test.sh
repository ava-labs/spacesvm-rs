PORT=:8080
GRPC_PORT=:8081

#hash of minikvvm
MINI_KVVM_HASH=qBnAKUQ2mxiB1JdqsPPU7Ufuj1XmPLpnPTRvZEpkYZBmK6UjE

kill -9 $(lsof -t -i$PORT)

# build new version of mini-kvvm
rm ${AVALANCHEGO_PLUGIN_PATH}/${MINI_KVVM_HASH}
cargo build \
--release \
--bin mini-kvvm
cp target/release/mini-kvvm ${AVALANCHEGO_PLUGIN_PATH}/${MINI_KVVM_HASH}

#Start the network runner
avalanche-network-runner server \
--log-level debug \
--port=$PORT \
--grpc-gateway-port=$GRPC_PORT &
NETWORK_RUNNER_PID=${!}
sleep 5

#Make a new instance of mini-kvvm
avalanche-network-runner control start \
--log-level all \
--endpoint="0.0.0.0:8080" \
--number-of-nodes=5 \
--avalanchego-path ${AVALANCHEGO_EXEC_PATH} \
--plugin-dir ${AVALANCHEGO_PLUGIN_PATH} \
--blockchain-specs '[{"vm_name":"minikvvm","genesis":"/tmp/mini-kvvm.genesis.json"}]' \
