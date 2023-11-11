use super::{TransactionError, TxContext};
use crate::contracts::RESERVE_ERGO_TREE;
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use ergo_lib::ergotree_ir::chain::{ergo_box::NonMandatoryRegisterId, token::Token};
use ergo_lib::wallet::{box_selector::BoxSelection, tx_builder::TxBuilder};

pub struct MintReserveOpt {
    pub pk: EcPoint,
    pub amount: u64,
}

pub fn mint_reserve_transaction(
    opts: MintReserveOpt,
    inputs: BoxSelection<ErgoBox>,
    context: TxContext,
) -> Result<UnsignedTransaction, TransactionError> {
    let mut reserve_box_builder = ErgoBoxCandidateBuilder::new(
        opts.amount.try_into()?,
        RESERVE_ERGO_TREE.clone(),
        context.current_height,
    );
    let nft_id = inputs
        .boxes
        .get(0)
        .ok_or_else(|| {
            TransactionError::MissingBox(
                "mint_reserve_transaction: failed to find input box required to mint nft"
                    .to_string(),
            )
        })?
        .box_id();
    let nft = Token {
        token_id: nft_id.into(),
        amount: 1.try_into()?,
    };
    reserve_box_builder.add_token(nft);
    reserve_box_builder.set_register_value(NonMandatoryRegisterId::R4, opts.pk.into());
    Ok(TxBuilder::new(
        inputs,
        vec![reserve_box_builder.build()?],
        context.current_height,
        context.fee.try_into()?,
        NetworkAddress::try_from(context.change_address)?.address(),
    )
    .build()?)
}
