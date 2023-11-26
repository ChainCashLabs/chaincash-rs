use super::{TransactionError, TxContext};
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::ergo_chain_types::{ADDigest, EcPoint};
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::NonMandatoryRegisterId;
use ergo_lib::ergotree_ir::chain::{ergo_box::ErgoBox, token::Token};
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::avl_tree_data::{AvlTreeData, AvlTreeFlags};
use ergo_lib::wallet::box_selector::BoxSelection;
use ergo_lib::wallet::tx_builder::TxBuilder;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MintNoteRequest {
    pub owner_public_key_hex: String,
    // the amount of nanoergs to deposit in the box
    // just the minimum amount required for boxes?
    pub box_nanoergs: u64,
    // Currently the amount in mg of gold
    // later will refactor to support more nominations
    // this is represented by a token at index 0 on the box
    pub gold_amount_mg: u64,
}

// TODO check serialized bytes match scala - not sure if avl tree digest is correct as default
pub fn mint_note_transaction(
    request: MintNoteRequest,
    note_tree: ErgoTree,
    inputs: BoxSelection<ErgoBox>,
    context: TxContext,
) -> Result<UnsignedTransaction, TransactionError> {
    let owner_pk =
        EcPoint::try_from(request.owner_public_key_hex).map_err(TransactionError::Parsing)?;
    let avl_flags = AvlTreeFlags::new(true, false, false);
    let avl_tree = AvlTreeData {
        digest: ADDigest::zero(),
        tree_flags: avl_flags,
        key_length: 32,
        value_length_opt: None,
    };
    let token_id = inputs
        .boxes
        .get(0)
        .ok_or_else(|| {
            TransactionError::MissingBox(
                "failed to find input box required to mint nft".to_string(),
            )
        })?
        .box_id();
    let token = Token {
        token_id: token_id.into(),
        amount: request.gold_amount_mg.try_into()?,
    };
    let mut note_box_builder = ErgoBoxCandidateBuilder::new(
        request.box_nanoergs.try_into()?,
        note_tree,
        context.current_height,
    );
    note_box_builder.add_token(token);
    note_box_builder.set_register_value(NonMandatoryRegisterId::R4, avl_tree.into());
    note_box_builder.set_register_value(NonMandatoryRegisterId::R5, owner_pk.into());
    note_box_builder.set_register_value(NonMandatoryRegisterId::R6, 0i64.into());
    Ok(TxBuilder::new(
        inputs,
        vec![note_box_builder.build()?],
        context.current_height,
        context.fee.try_into()?,
        NetworkAddress::try_from(context.change_address)?.address(),
    )
    .build()?)
}
