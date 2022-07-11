if ! [[ "$0" =~ scripts/start_server.sh ]]; then
  echo "must be run from repository root"
  exit 255
fi

cargo run --bin server-jsonrpc