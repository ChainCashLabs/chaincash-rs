pub mod reserves;

use crate::{contracts::RESERVE_ERGO_TREE, NanoErg};
use ergo_lib::{
    chain::{ergo_box::box_builder::ErgoBoxCandidateBuilder, transaction::TxId},
    ergo_chain_types::EcPoint,
    ergotree_ir::chain::{
        address::NetworkAddress,
        ergo_box::{box_value::BoxValue, ErgoBox, NonMandatoryRegisterId},
        token::TokenId,
        token::{Token, TokenAmount},
    },
    wallet::{
        box_selector::{BoxSelection, BoxSelector, SimpleBoxSelector},
        tx_builder::{TxBuilder, SUGGESTED_TX_FEE},
    },
};
use ergo_node_interface::NodeInterface;

#[derive(Clone)]
pub struct TransactionService {
    // TODO: inputs cache so we dont need to fetch after each tx build
    node: NodeInterface,
    fee: NanoErg,
}

struct TxContext {
    current_height: u32,
    change_address: String,
    fee: NanoErg,
}

impl TransactionService {
    pub fn new(node: NodeInterface) -> Self {
        Self::with_fee(node, SUGGESTED_TX_FEE().as_u64().clone())
    }

    pub fn with_fee(node: NodeInterface, fee: NanoErg) -> Self {
        Self { node, fee }
    }

    fn box_selection_with_amount(&self, amount: NanoErg) -> BoxSelection<ErgoBox> {
        let inputs = self.node.unspent_boxes_with_min_total(amount).unwrap();
        // kinda irrelevant since we already have suitable boxes but box selectors required by txbuilder
        SimpleBoxSelector::new()
            .select(inputs, amount.try_into().unwrap(), &[])
            .unwrap()
    }

    // TODO: handle request failures
    // handle wallet uninitialized/locked
    // handle no change address
    fn get_tx_ctx(&self) -> TxContext {
        let wallet_status = self.node.wallet_status().unwrap();

        TxContext {
            current_height: self.node.current_block_height().unwrap() as u32,
            change_address: wallet_status.change_address.unwrap(),
            fee: self.fee,
        }
    }

    pub fn create_reserve(&self, pk: EcPoint, amount: NanoErg) -> TxId {
        // TODO: get utxos from a cache
        let ctx = self.get_tx_ctx();
        let selected_inputs = self.box_selection_with_amount(amount + ctx.fee);
        let mut reserve_box_builder = ErgoBoxCandidateBuilder::new(
            BoxValue::try_from(amount).unwrap(),
            RESERVE_ERGO_TREE.clone(),
            ctx.current_height,
        );
        reserve_box_builder.add_token(Token {
            token_id: TokenId::from(selected_inputs.boxes.get(0).unwrap().box_id()),
            amount: TokenAmount::try_from(1).unwrap(),
        });
        reserve_box_builder.set_register_value(NonMandatoryRegisterId::R4, pk.into());
        let unsigned_tx = TxBuilder::new(
            selected_inputs,
            vec![reserve_box_builder.build().unwrap()],
            ctx.current_height,
            BoxValue::try_from(ctx.fee).unwrap(),
            NetworkAddress::try_from(ctx.change_address)
                .unwrap()
                .address(),
        )
        .build()
        .unwrap();
        self.node.sign_and_submit_transaction(&unsigned_tx).unwrap()
    }
}
