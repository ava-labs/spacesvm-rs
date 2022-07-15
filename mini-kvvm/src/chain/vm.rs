use std::io::Result;

use avalanche_proto::{
    appsender::app_sender_client::AppSenderClient, messenger::messenger_client::MessengerClient,
};
use avalanche_types::{
    ids::{set::Set, Id},
    rpcchainvm::database::manager::Manager,
};
use tonic::transport::Channel;

use crate::chain::block::StatelessBlock;
use crate::chain::genesis::Genesis;
use crate::chain::txn::TransactionInterior;

pub struct Context {
    pub recent_block_ids: Set,
    pub recent_tx_ids: Set,
    pub recent_load_units: u64,
}

#[tonic::async_trait]
pub trait Vm:
    avalanche_types::rpcchainvm::block::ChainVm + crate::chain::network::PushNetwork
{
    /// Initialize the Vm.
    /// [vm_inner]:
    /// [ctx]: Metadata about the Vm
    /// [db_manager]: Manager of the database this Vm will run on
    /// [genesis_bytes]: Byte-encoding of genesis data for the Vm.
    ///                  This is data the Vm uses to intialize its
    ///                  state.
    /// [upgrade_bytes]: Byte-encoding of update data
    /// [config_bytes]: Byte-encoding of configuration data
    /// [to_engine]: Channel used to send messages to the consensus engine
    /// [fxs]: Feature extensions that attach to this Vm.
    /// [app_sender]: Channel used to send app requests
    async fn initialize(
        &self,
        ctx: Option<avalanche_types::vm::context::Context>,
        db_manager: Box<dyn Manager>,
        genesis_bytes: &[u8],
        upgrade_bytes: &[u8],
        config_bytes: &[u8],
        to_engine: &MessengerClient<Channel>,
        _fxs: (),
        app_sender: &AppSenderClient<Channel>,
    ) -> Result<()>;

    async fn get_block(&self, id: Id) -> Result<StatelessBlock>;
    async fn genesis(&self) -> Genesis;
    async fn is_bootstrapped(&self);
    async fn state(&self)
        -> Box<dyn avalanche_types::rpcchainvm::database::Database + Send + Sync>;
    async fn get_stateless_block(&self, block_id: Id) -> Result<StatelessBlock>;
    async fn execution_context(
        &self,
        current_time: u64,
        parent: &StatelessBlock,
    ) -> Result<Context>;

    // Push Network
    async fn send_txs(&self, txs: Vec<TransactionInterior>) -> Result<()>;
    async fn gossip_new_tx(&self, new_tx: Vec<TransactionInterior>) -> Result<()>;
    async fn regossip_tx(&self) -> Result<()>;
}
