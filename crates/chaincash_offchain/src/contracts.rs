use ergo_lib::ergoscript_compiler::compiler::compile;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use once_cell::sync::Lazy;

pub(crate) static RESERVE_CONTRACT: &str =
    include_str!("../../../contracts/chaincash/contracts/onchain/reserve.es");
// Base58 hash of reserve contract
pub(crate) static RESERVE_CONTRACT_HASH: Lazy<String> =
    Lazy::new(|| bs58::encode(RESERVE_CONTRACT).into_string());
// Not currently able to compile with sigma-rust, using node api instead
pub(crate) static RESERVE_ERGO_TREE: Lazy<ErgoTree> =
    Lazy::new(|| compile(RESERVE_CONTRACT, Default::default()).unwrap());

pub(crate) static RECEIPT_CONTRACT: &str =
    include_str!("../../../contracts/chaincash/contracts/onchain/receipt.es");
pub(crate) static RECEIPT_CONTRACT_HASH: Lazy<String> =
    Lazy::new(|| bs58::encode(RECEIPT_CONTRACT).into_string());

pub(crate) static NOTE_CONTRACT: &str =
    include_str!("../../../contracts/chaincash/contracts/onchain/note.es");
// Not currently able to compile with sigma-rust, using node api instead
pub(crate) static NOTE_ERGO_TREE: Lazy<ErgoTree> =
    Lazy::new(|| compile(NOTE_CONTRACT, Default::default()).unwrap());
