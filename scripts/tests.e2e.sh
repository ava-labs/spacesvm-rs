#!/usr/bin/env bash
set -e

# ./scripts/tests.e2e.sh 1.7.11
if ! [[ "$0" =~ scripts/tests.e2e.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

#################################
# download avalanche-network-runner
# https://github.com/ava-labs/avalanche-network-runner
# TODO: use "go install -v github.com/ava-labs/avalanche-network-runner/cmd/avalanche-network-runner@v${NETWORK_RUNNER_VERSION}"
GOOS=$(go env GOOS) # ensures that the compatible network runner version is downloaded for this machine
NETWORK_RUNNER_VERSION=1.1.0
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
--grpc-gateway-port=":12343" &
NETWORK_RUNNER_PID=${!}
sleep 5 # sleep to ensure that network runner initializes before e2e tests begin

#################################
# do not run in parallel, to run in sequence
echo "running e2e tests"
NETWORK_RUNNER_GRPC_ENDPOINT=http://127.0.0.1:12342 \
RUST_LOG=debug \
cargo test --all-features --package e2e -- --show-output --nocapture

#################################
# "e2e.test" already terminates the cluster for "test" mode
# just in case tests are aborted, manually terminate them again
echo "network-runner RPC server was running on NETWORK_RUNNER_PID ${NETWORK_RUNNER_PID} as test mode; terminating the process..."
pkill -P ${NETWORK_RUNNER_PID} || true
kill -2 ${NETWORK_RUNNER_PID}
