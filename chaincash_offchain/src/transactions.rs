pub mod reserves;

use crate::NanoErg;
use ergo_lib::{
    chain::transaction::TxId,
    ergo_chain_types::EcPoint,
    ergotree_ir::chain::ergo_box::ErgoBox,
    wallet::{
        box_selector::{BoxSelection, BoxSelector, SimpleBoxSelector},
        tx_builder::SUGGESTED_TX_FEE,
    },
};
use ergo_node_interface::NodeInterface;
use thiserror::Error;

use self::reserves::mint_reserve_tx;

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("unable to get change address from wallet")]
    ChangeAddress,
}

#[derive(Clone)]
pub struct TransactionService {
    node: NodeInterface,
    fee: NanoErg,
}

pub struct TxContext {
    current_height: u32,
    change_address: String,
    fee: NanoErg,
}

impl TransactionService {
    pub fn new(node: NodeInterface) -> Self {
        Self::with_fee(node, *SUGGESTED_TX_FEE().as_u64())
    }

    pub fn with_fee(node: NodeInterface, fee: NanoErg) -> Self {
        Self { node, fee }
    }

    fn box_selection_with_amount(
        &self,
        amount: NanoErg,
    ) -> Result<BoxSelection<ErgoBox>, crate::Error> {
        let inputs = self.node.unspent_boxes_with_min_total(amount)?;
        // kinda irrelevant since we already have suitable boxes but box selectors required by txbuilder
        Ok(SimpleBoxSelector::new()
            .select(inputs, amount.try_into().unwrap(), &[])
            .unwrap())
    }

    // TODO: handle wallet uninitialized/locked
    fn get_tx_ctx(&self) -> Result<TxContext, crate::Error> {
        let wallet_status = self.node.wallet_status()?;

        Ok(TxContext {
            current_height: self.node.current_block_height()? as u32,
            change_address: wallet_status
                .change_address
                .ok_or(TransactionError::ChangeAddress)?,
            fee: self.fee,
        })
    }

    pub fn mint_reserve(&self, pk: EcPoint, amount: NanoErg) -> Result<TxId, crate::Error> {
        let ctx = self.get_tx_ctx()?;
        let selected_inputs = self.box_selection_with_amount(amount + ctx.fee)?;
        let unsigned_tx = mint_reserve_tx(reserves::MintReserveOpt {
            pk,
            amount,
            ctx,
            inputs: selected_inputs,
        });
        Ok(self.node.sign_and_submit_transaction(&unsigned_tx)?)
    }
}
