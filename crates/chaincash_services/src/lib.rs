use chaincash_predicate::predicates::Predicate;
use chaincash_store::ChainCashStore;
use compiler::Compiler;
use ergo_client::node::NodeClient;
use transaction::TransactionService;

pub mod compiler;
pub mod scanner;
pub mod transaction;

#[derive(Clone)]
pub struct ServerState {
    pub store: ChainCashStore,
    pub node: NodeClient,
    compiler: Compiler,
    pub predicates: Vec<Predicate>,
}

impl ServerState {
    pub fn new(node: NodeClient, store: ChainCashStore, predicates: Vec<Predicate>) -> Self {
        ServerState {
            compiler: Compiler::new(node.clone()),
            node,
            store,
            predicates,
        }
    }
    pub fn tx_service(&self) -> TransactionService {
        TransactionService::new(&self.node, &self.store, &self.compiler)
    }
}
