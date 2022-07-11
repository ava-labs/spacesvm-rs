

pub struct Vm {
    inner: Arc<RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
    closed: AtomicBool,
}

// Database is local scope which allows the following usage.
// database::memdb::Database::new()
impl Vm {
    pub fn new() -> Box<dyn crate::rpcchainvm::database::Database + Send + Sync> {
        let state: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
        Box::new(Database {
            inner: Arc::new(RwLock::new(state)),
            closed: AtomicBool::new(false),
        })
    }
}

/// pub trait ChainVm: Vm + Getter + Parser {}
impl crate::rpcchainvm::block::ChainVm for Vm {}

#[tonic::async_trait]
impl crate::rpcchainvm::block::Getter for Database {}
impl crate::rpcchainvm::block::Parser for Database {}
impl crate::rpcchainvm::common::Vm for Database {}


/// ...