use ergo_lib::ergoscript_compiler::compiler::compile;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use once_cell::sync::Lazy;

pub(crate) static RESERVE_ERGO_TREE: Lazy<ErgoTree> = Lazy::new(|| {
    let s = include_str!("../../../contracts/chaincash/contracts/onchain/reserve.es");
    compile(s, Default::default()).unwrap()
});

pub(crate) static NOTE_ERGO_TREE: Lazy<ErgoTree> = Lazy::new(|| {
    let s = include_str!("../../../contracts/chaincash/contracts/onchain/note.es");
    compile(s, Default::default()).unwrap()
});
