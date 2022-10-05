use std::io::Result;

use avalanche_types::rpcchainvm;
use tokio::sync::broadcast;

use crate::vm;

/// Data required to start the Vm.
pub struct Bootstrap {
    /// Name of this Vm instance.
    pub name: String,

    /// Vm log level.
    pub log_level: String,

    /// Vm version.
    pub version: semver::Version,

    pub testing: bool,
}

pub type Runner = Bootstrap;

impl Runner {
    pub fn new() -> Runner {
        Runner {
            name: String::from("Vm"),
            log_level: "INFO".to_string(),
            version: semver::Version::parse("0.0.0").unwrap(),
            testing: false,
        }
    }

    /// Default is Vm.
    pub fn name(mut self, v: &str) -> Runner {
        self.name = v.to_owned();
        self
    }

    /// Default is debug.
    pub fn log_level(mut self, v: &str) -> Runner {
        self.log_level = v.to_owned();
        self
    }

    /// Default is v0.0.0.
    pub fn version(mut self, v: &str) -> Runner {
        self.version = semver::Version::parse(&v.to_owned()).unwrap();
        self
    }

    /// Default is false.
    pub fn testing(mut self, v: bool) -> Runner {
        self.testing = v;
        self
    }

    /// Starts the Vm and blocks until stop signal is sent.
    pub async fn run(self) -> Result<()> {
        let (stop_ch_tx, stop_ch_rx): (broadcast::Sender<()>, broadcast::Receiver<()>) =
            broadcast::channel(1);
        let vm = vm::ChainVm::new(Bootstrap {
            name: self.name,
            log_level: self.log_level,
            version: self.version,
            testing: self.testing,
        });

        let vm_server =
            avalanche_types::rpcchainvm::vm::server::Server::new(Box::new(vm), stop_ch_tx);

        // Blocks until stop channel signal is received.
        rpcchainvm::plugin::serve(vm_server, stop_ch_rx)
            .await
            .expect("failed to start gRPC server");

        Ok(())
    }
}
