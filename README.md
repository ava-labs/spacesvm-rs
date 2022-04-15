
![Github Actions](https://github.com/ava-labs/mini-kvvm-rs/actions/workflows/test-and-release.yml/badge.svg)

# mini-kvvm-rs

Mini key-value store VM in Rust for Avalanche

## Rust Version

`mini-kvvm-rs` currently works on Rust `1.60+` and requires support for the `2021` edition.

```bash
cd ${HOME}/go/src/github.com/ava-labs/subnet-cli
go install -v .
subnet-cli create VMID minikvvmrs
# qBnAKUQ2mxjMHCneWjq5nFuhntoWrsKsCjaYSouFjpuCB2o5d

cd ${HOME}/mini-kvvm-rs
./scripts/build.x86_64-linux-musl.sh
cp \
./target/x86_64-unknown-linux-musl/release/mini-kvvm-rs \
${HOME}/go/src/github.com/ava-labs/avalanchego/build/plugins/qBnAKUQ2mxjMHCneWjq5nFuhntoWrsKsCjaYSouFjpuCB2o5d
```
