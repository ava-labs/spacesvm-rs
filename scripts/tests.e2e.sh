#!/usr/bin/env bash
set -e

# created a new VMID from minikvvm
VMID="qBnAKUQ2mxiB1JdqsPPU7Ufuj1XmPLpnPTRvZEpkYZBmK6UjE"
VM_NAME="minikvvm"
VM_GENESIS_PATH="/tmp/minikvvm.genesis.json"

# ./scripts/tests.e2e.sh 1.7.10
if ! [[ "$0" =~ scripts/tests.e2e.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

AVALANCHEGO_VERSION=$1
if [[ -z "${AVALANCHEGO_VERSION}" ]]; then
  echo "Missing avalanchego version argument!"
  echo "Usage: ${0} [AVALANCHEGO_VERSION]" >> /dev/stderr
  exit 255
fi

echo "Running with:"
echo AVALANCHEGO_VERSION: ${AVALANCHEGO_VERSION}

############################
# download avalanchego
# https://github.com/ava-labs/avalanchego/releases
GOARCH=$(go env GOARCH)
GOOS=$(go env GOOS)
DOWNLOAD_URL=https://github.com/ava-labs/avalanchego/releases/download/v${AVALANCHEGO_VERSION}/avalanchego-linux-${GOARCH}-v${AVALANCHEGO_VERSION}.tar.gz
DOWNLOAD_PATH=/tmp/avalanchego.tar.gz
if [[ ${GOOS} == "darwin" ]]; then
  DOWNLOAD_URL=https://github.com/ava-labs/avalanchego/releases/download/v${AVALANCHEGO_VERSION}/avalanchego-macos-v${AVALANCHEGO_VERSION}.zip
  DOWNLOAD_PATH=/tmp/avalanchego.zip
fi

rm -rf /tmp/avalanchego-v${AVALANCHEGO_VERSION}
rm -f ${DOWNLOAD_PATH}

echo "downloading avalanchego ${AVALANCHEGO_VERSION} at ${DOWNLOAD_URL}"
curl -L ${DOWNLOAD_URL} -o ${DOWNLOAD_PATH}

echo "extracting downloaded avalanchego"
if [[ ${GOOS} == "linux" ]]; then
  tar xzvf ${DOWNLOAD_PATH} -C /tmp
elif [[ ${GOOS} == "darwin" ]]; then
  unzip ${DOWNLOAD_PATH} -d /tmp/avalanchego-build
  mv /tmp/avalanchego-build/build /tmp/avalanchego-v${AVALANCHEGO_VERSION}
fi
find /tmp/avalanchego-v${AVALANCHEGO_VERSION}

AVALANCHEGO_PATH=/tmp/avalanchego-v${AVALANCHEGO_VERSION}/avalanchego
AVALANCHEGO_PLUGIN_DIR=/tmp/avalanchego-v${AVALANCHEGO_VERSION}/plugins

rm -f "${AVALANCHEGO_PLUGIN_DIR}/${VMID}"

echo "compile mini-kvvm and install to plugin dir"
cargo build --release --bin mini-kvvm
mv ./target/release/mini-kvvm "${AVALANCHEGO_PLUGIN_DIR}/${VMID}"

cat <<EOF > ${VM_GENESIS_PATH}
{
  "author": "foo",
  "welcome_message": "bar"
}
EOF

cat ${VM_GENESIS_PATH}

#################################
# download avalanche-network-runner
# https://github.com/ava-labs/avalanche-network-runner
# TODO: use "go install -v github.com/ava-labs/avalanche-network-runner/cmd/avalanche-network-runner@v${NETWORK_RUNNER_VERSION}"
NETWORK_RUNNER_VERSION=1.0.16-beta
DOWNLOAD_PATH=/tmp/avalanche-network-runner.tar.gz
DOWNLOAD_URL=https://github.com/ava-labs/avalanche-network-runner/releases/download/v${NETWORK_RUNNER_VERSION}/avalanche-network-runner_${NETWORK_RUNNER_VERSION}_linux_amd64.tar.gz
if [[ ${GOOS} == "darwin" ]]; then
  DOWNLOAD_URL=https://github.com/ava-labs/avalanche-network-runner/releases/download/v${NETWORK_RUNNER_VERSION}/avalanche-network-runner_${NETWORK_RUNNER_VERSION}_darwin_amd64.tar.gz
fi

rm -f ${DOWNLOAD_PATH}
rm -f /tmp/avalanche-network-runner

echo "downloading avalanche-network-runner ${NETWORK_RUNNER_VERSION} at ${DOWNLOAD_URL}"
curl -L ${DOWNLOAD_URL} -o ${DOWNLOAD_PATH}

echo "extracting downloaded avalanche-network-runner"
tar xzvf ${DOWNLOAD_PATH} -C /tmp
/tmp/avalanche-network-runner -h

#################################
# run "avalanche-network-runner" server
echo "launch avalanche-network-runner in the background"
/tmp/avalanche-network-runner \
server \
--log-level debug \
--port=":12342" \
--disable-grpc-gateway &
NETWORK_RUNNER_PID=${!}

#################################
# do not run in parallel, to run in sequence
echo "running e2e tests"
NETWORK_RUNNER_GRPC_ENDPOINT=http://127.0.0.1:12342 \
NETWORK_RUNNER_AVALANCHEGO_PATH=${AVALANCHEGO_PATH} \
NETWORK_RUNNER_WHITELISTED_SUBNETS=${VMID} \
NETWORK_RUNNER_PLUGIN_DIR_PATH=${AVALANCHEGO_PLUGIN_DIR} \
NETWORK_RUNNER_CUSTOM_VM="${VM_NAME}=${VM_GENESIS_PATH}" \
RUST_LOG=debug \
cargo test --all-features --package e2e -- --show-output --nocapture

#################################
# "e2e.test" already terminates the cluster for "test" mode
# just in case tests are aborted, manually terminate them again
echo "network-runner RPC server was running on NETWORK_RUNNER_PID ${NETWORK_RUNNER_PID} as test mode; terminating the process..."
pkill -P ${NETWORK_RUNNER_PID} || true
kill -2 ${NETWORK_RUNNER_PID}
