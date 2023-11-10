use ergo_lib::{
    chain::{
        ergo_box::box_builder::ErgoBoxCandidateBuilder, transaction::unsigned::UnsignedTransaction,
    },
    ergo_chain_types::EcPoint,
    ergotree_ir::chain::{
        address::NetworkAddress,
        ergo_box::{box_value::BoxValue, ErgoBox, NonMandatoryRegisterId},
        token::{Token, TokenAmount, TokenId},
    },
    wallet::{box_selector::BoxSelection, tx_builder::TxBuilder},
};

use crate::contracts::RESERVE_ERGO_TREE;

use super::TxContext;

pub struct MintReserveOpt {
    pub pk: EcPoint,
    pub amount: u64,
    pub ctx: TxContext,
    pub inputs: BoxSelection<ErgoBox>,
}

pub fn mint_reserve_tx(opts: MintReserveOpt) -> UnsignedTransaction {
    let mut reserve_box_builder = ErgoBoxCandidateBuilder::new(
        BoxValue::try_from(opts.amount).unwrap(),
        RESERVE_ERGO_TREE.clone(),
        opts.ctx.current_height,
    );
    reserve_box_builder.add_token(Token {
        token_id: TokenId::from(opts.inputs.boxes.get(0).unwrap().box_id()),
        amount: TokenAmount::try_from(1).unwrap(),
    });
    reserve_box_builder.set_register_value(NonMandatoryRegisterId::R4, opts.pk.into());
    TxBuilder::new(
        opts.inputs,
        vec![reserve_box_builder.build().unwrap()],
        opts.ctx.current_height,
        BoxValue::try_from(opts.ctx.fee).unwrap(),
        NetworkAddress::try_from(opts.ctx.change_address)
            .unwrap()
            .address(),
    )
    .build()
    .unwrap()
}
