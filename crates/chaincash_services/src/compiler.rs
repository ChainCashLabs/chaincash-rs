use chaincash_offchain::contracts::{NOTE_CONTRACT, RECEIPT_CONTRACT, RESERVE_CONTRACT};
use ergo_client::node::{NodeClient, NodeError};
use ergo_lib::{
    ergo_chain_types::blake2b256_hash,
    ergotree_ir::{ergo_tree::ErgoTree, serialization::SigmaSerializable},
};
use tokio::sync::OnceCell;

#[derive(Clone)]
pub struct Compiler {
    node: NodeClient,
    reserve_contract: OnceCell<ErgoTree>,
    note_contract: OnceCell<ErgoTree>,
    receipt_contract: OnceCell<ErgoTree>,
}

impl Compiler {
    pub fn new(node: NodeClient) -> Self {
        Compiler {
            node,
            reserve_contract: OnceCell::new(),
            note_contract: OnceCell::new(),
            receipt_contract: OnceCell::new(),
        }
    }
    pub async fn reserve_contract(&self) -> Result<&ErgoTree, NodeError> {
        self.reserve_contract
            .get_or_try_init(|| async {
                self.node
                    .extensions()
                    .compile_contract(RESERVE_CONTRACT)
                    .await
            })
            .await
    }
    pub async fn receipt_contract(&self) -> Result<&ErgoTree, NodeError> {
        self.receipt_contract
            .get_or_try_init(|| async {
                let reserve_tree_bytes = self
                    .reserve_contract()
                    .await?
                    .sigma_serialize_bytes()
                    .unwrap();
                let reserve_hash =
                    bs58::encode(blake2b256_hash(&reserve_tree_bytes[1..])).into_string();
                let receipt_contract =
                    RECEIPT_CONTRACT.replace("$reserveContractHash", &reserve_hash);
                self.node
                    .extensions()
                    .compile_contract(&receipt_contract)
                    .await
            })
            .await
    }
    pub async fn note_contract(&self) -> Result<&ErgoTree, NodeError> {
        self.note_contract
            .get_or_try_init(|| async {
                let reserve_tree_bytes = self
                    .reserve_contract()
                    .await?
                    .sigma_serialize_bytes()
                    .unwrap();
                let reserve_hash =
                    bs58::encode(blake2b256_hash(&reserve_tree_bytes[1..])).into_string();
                let receipt_tree_bytes = self
                    .receipt_contract()
                    .await?
                    .sigma_serialize_bytes()
                    .unwrap();
                let receipt_hash =
                    bs58::encode(blake2b256_hash(&receipt_tree_bytes[1..])).into_string();
                let note_contract = NOTE_CONTRACT
                    .replace("$reserveContractHash", &reserve_hash)
                    .replace("$receiptContractHash", &receipt_hash);
                self.node
                    .extensions()
                    .compile_contract(&note_contract)
                    .await
            })
            .await
    }
}
