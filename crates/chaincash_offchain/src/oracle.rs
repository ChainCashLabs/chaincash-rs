use ergo_lib::{ergo_chain_types::Digest32, ergotree_ir::chain::token::TokenId};

pub const GOLD_ORACLE_NFT: &str =
    "3c45f29a5165b030fdb5eaf5d81f8108f9d8f507b31487dd51f4ae08fe07cf4a";
pub const BUYBACK_NFT: &str = "bf24ed4af7eb5a7839c43aa6b240697d81b196120c837e1a941832c266d3755c";
const GOLD_ORACLE_NFT_TESTNET: &str =
    "a7271cbaea40c8718ef568ebbda125b195207c597b7d53d14873f0b521d4f6d1";
const BUYBACK_NFT_TESTNET: &str =
    "8fc353ac5bf8411b757180001e15fb87840a37ebd2e133fe70dce857fc526a19";

pub fn oracle_nft(is_mainnet: bool) -> TokenId {
    if is_mainnet {
        Digest32::try_from(GOLD_ORACLE_NFT.to_string())
            .unwrap()
            .into()
    } else {
        Digest32::try_from(GOLD_ORACLE_NFT_TESTNET.to_string())
            .unwrap()
            .into()
    }
}

pub fn buyback_nft(is_mainnet: bool) -> TokenId {
    if is_mainnet {
        Digest32::try_from(BUYBACK_NFT.to_string()).unwrap().into()
    } else {
        Digest32::try_from(BUYBACK_NFT_TESTNET.to_string())
            .unwrap()
            .into()
    }
}
