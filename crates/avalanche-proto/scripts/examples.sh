
#!/usr/bin/env bash
set -xue

if ! [[ "$0" =~ ./scripts/examples.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

# cleanup port
fuser -k 50051/tcp || true

cargo run --example server &
sleep 1
cargo run --example client