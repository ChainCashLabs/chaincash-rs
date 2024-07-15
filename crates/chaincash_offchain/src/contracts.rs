use ergo_lib::ergoscript_compiler::compiler::compile;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use once_cell::sync::Lazy;

pub static RESERVE_CONTRACT: &str =
    include_str!("../../../contracts/chaincash/contracts/onchain/reserve.es");
// Not currently able to compile with sigma-rust, using node api instead
pub static RESERVE_ERGO_TREE: Lazy<ErgoTree> =
    Lazy::new(|| compile(RESERVE_CONTRACT, Default::default()).unwrap());

pub static RECEIPT_CONTRACT: &str =
    include_str!("../../../contracts/chaincash/contracts/onchain/receipt.es");

pub static NOTE_CONTRACT: &str =
    include_str!("../../../contracts/chaincash/contracts/onchain/note.es");
// Not currently able to compile with sigma-rust, using node api instead
pub static NOTE_ERGO_TREE: Lazy<ErgoTree> =
    Lazy::new(|| compile(NOTE_CONTRACT, Default::default()).unwrap());
