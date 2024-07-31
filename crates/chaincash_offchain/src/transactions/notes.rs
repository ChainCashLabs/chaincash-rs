use super::{TransactionError, TxContext};
use crate::boxes::{Note, ReserveBoxSpec};
use crate::note_history::NoteHistory;

use ergo_avltree_rust::authenticated_tree_ops::AuthenticatedTreeOps;
use ergo_avltree_rust::batch_avl_prover::BatchAVLProver;
use ergo_avltree_rust::batch_node::{AVLTree, Node, NodeHeader};
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::transaction::ergo_transaction::ErgoTransaction;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::{DataInput, Transaction};
use ergo_lib::ergo_chain_types::{ADDigest, EcPoint};
use ergo_lib::ergotree_interpreter::sigma_protocol::prover::ContextExtension;
use ergo_lib::ergotree_interpreter::sigma_protocol::wscalar::Wscalar;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::{ErgoBox, ErgoBoxCandidate, NonMandatoryRegisterId};
use ergo_lib::ergotree_ir::chain::token::Token;
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::ergotree_ir::mir::avl_tree_data::{AvlTreeData, AvlTreeFlags};
use ergo_lib::wallet::box_selector::{BoxSelection, BoxSelector, SimpleBoxSelector};
use ergo_lib::wallet::tx_builder::TxBuilder;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MintNoteRequest {
    pub owner_public_key_hex: String,
    // Currently the amount in mg of gold
    // later will refactor to support more nominations
    // this is represented by a token at index 0 on the box
    pub gold_amount_mg: u64,
}

pub struct MintNoteResponse<T: ErgoTransaction> {
    pub note: Note,
    pub transaction: T,
}

pub type SignedMintNoteResponse = MintNoteResponse<Transaction>;

pub fn mint_note_transaction(
    request: MintNoteRequest,
    note_tree: ErgoTree,
    inputs: BoxSelection<ErgoBox>,
    context: TxContext,
) -> Result<MintNoteResponse<UnsignedTransaction>, TransactionError> {
    let owner_pk =
        EcPoint::try_from(request.owner_public_key_hex).map_err(TransactionError::Parsing)?;
    let prover = BatchAVLProver::new(
        AVLTree::new(
            |digest| Node::LabelOnly(NodeHeader::new(Some(*digest), None)),
            32,
            None,
        ),
        true,
    );
    let digest: ADDigest = prover
        .digest()
        .unwrap()
        .into_iter()
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let avl_flags = AvlTreeFlags::new(true, false, false);
    let avl_tree = AvlTreeData {
        digest,
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
    let mut note_box_builder =
        ErgoBoxCandidateBuilder::new(BoxValue::SAFE_USER_MIN, note_tree, context.current_height);
    note_box_builder.add_token(token);
    note_box_builder.set_register_value(NonMandatoryRegisterId::R4, avl_tree.into());
    note_box_builder.set_register_value(NonMandatoryRegisterId::R5, owner_pk.into());
    note_box_builder.set_register_value(NonMandatoryRegisterId::R6, 0i64.into());
    let unsigned_transaction = TxBuilder::new(
        inputs,
        vec![note_box_builder.build()?],
        context.current_height,
        context.fee.try_into()?,
        NetworkAddress::try_from(context.change_address)?.address(),
    )
    .build()?;
    let note = Note::new(
        unsigned_transaction.outputs().first().unwrap().clone(),
        NoteHistory::new(),
    )
    .unwrap();
    Ok(MintNoteResponse {
        note,
        transaction: unsigned_transaction,
    })
}

fn create_note_candidate(
    note: &Note,
    new_digest: AvlTreeData,
    recipient: EcPoint,
    token_amount: u64,
    height: u32,
) -> Result<ErgoBoxCandidate, TransactionError> {
    // Note value must be >= old note's value
    let mut box_candidate = ErgoBoxCandidateBuilder::new(
        note.ergo_box().value,
        note.ergo_box().ergo_tree.clone(),
        height,
    );
    box_candidate.add_token(Token {
        token_id: note.note_id,
        amount: token_amount.try_into()?,
    });
    box_candidate.set_register_value(NonMandatoryRegisterId::R4, new_digest.into());
    box_candidate.set_register_value(NonMandatoryRegisterId::R5, recipient.into());
    box_candidate.set_register_value(NonMandatoryRegisterId::R6, (note.length as i64 + 1).into());
    Ok(box_candidate.build()?)
}

pub struct SpendNoteResponse<T: ErgoTransaction> {
    pub transaction: T,
    pub recipient_note: Note,
    pub change_note: Option<Note>,
}

pub type SignedSpendNoteResponse = SpendNoteResponse<Transaction>;

pub fn spend_note_transaction(
    note: &Note,
    reserve: &ReserveBoxSpec,
    private_key: Wscalar,
    recipient: EcPoint,
    amount: u64,
    wallet_boxes: Vec<ErgoBox>,
    context: TxContext,
) -> Result<SpendNoteResponse<UnsignedTransaction>, TransactionError> {
    let change_amount =
        note.amount
            .as_u64()
            .checked_sub(amount)
            .ok_or(TransactionError::NoteAmountError {
                input_amount: *note.amount.as_u64(),
                output_amount: amount,
            })?;
    let has_change = change_amount > 0;

    // If there is a change note box must have atleast as many erg as the original note
    let erg_needed = note.ergo_box().value.as_u64() * has_change as u64 + context.fee;
    let box_selector = SimpleBoxSelector::new();
    let BoxSelection {
        boxes,
        change_boxes,
    } = box_selector.select(wallet_boxes, BoxValue::new(erg_needed)?, &[])?;
    let mut boxes = boxes.to_vec();
    boxes.push(note.ergo_box().clone());

    let mut new_history = note.history.clone();
    let signature = note.sign_note(reserve.identifier, private_key)?;
    let proof = new_history.add_commitment(signature.clone())?;
    let new_digest = new_history.to_avltree();

    let box_selection = BoxSelection {
        boxes: boxes.try_into().unwrap(),
        change_boxes,
    };
    let mut output_candidates = vec![create_note_candidate(
        note,
        new_digest.clone(),
        recipient,
        amount,
        context.current_height,
    )?];
    if has_change {
        output_candidates.push(create_note_candidate(
            note,
            new_digest,
            note.owner.clone(), // TODO: allow setting change address in request
            change_amount,
            context.current_height,
        )?)
    }
    let mut tx_builder = TxBuilder::new(
        box_selection,
        output_candidates,
        context.current_height,
        context.fee.try_into()?,
        NetworkAddress::try_from(context.change_address)?.address(),
    );
    let mut context_extension = ContextExtension::empty();
    context_extension.values.insert(0, 0i8.into()); // note output
    context_extension
        .values
        .insert(1, signature.signature.a().clone().into()); // signature a
    context_extension
        .values
        .insert(2, signature.signature.z_bytes().into()); // signature z
    context_extension.values.insert(3, proof.to_vec().into());
    if has_change {
        context_extension.values.insert(4, 1i8.into()); // change output index
    }
    tx_builder.set_context_extension(note.ergo_box().box_id(), context_extension);
    tx_builder.set_data_inputs(vec![DataInput {
        box_id: reserve.ergo_box().box_id(),
    }]);
    let transaction = tx_builder.build()?;

    let recipient_note = Note::new(
        transaction.outputs().get(0).unwrap().clone(),
        new_history.clone(),
    )
    .unwrap();
    let change_note = has_change
        .then(|| Note::new(transaction.outputs().get(1).unwrap().clone(), new_history).unwrap());
    Ok(SpendNoteResponse {
        transaction,
        recipient_note,
        change_note,
    })
}

#[cfg(test)]
mod test {
    use ergo_lib::{
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::{Address, NetworkAddress, NetworkPrefix},
            ergo_box::box_value::BoxValue,
        },
        wallet::{signing::TransactionContext, Wallet},
    };

    use crate::{
        test_util::{create_note, create_reserve, create_wallet_box, force_any_val},
        transactions::TxContext,
    };

    use super::spend_note_transaction;
    // Test spending a note with change output
    #[test]
    fn test_spend_note() {
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let note = create_note(&public_key, 10);
        let reserve = create_reserve(*public_key.clone());

        let mut wallet_boxes = vec![create_wallet_box(*public_key.clone(), 1_000_000_000)];
        let recipient = DlogProverInput::random().public_image().h;
        let note_response = spend_note_transaction(
            &note,
            &reserve,
            private_key.w.clone(),
            *recipient,
            8,
            wallet_boxes.clone(),
            TxContext {
                current_height: 0,
                change_address: NetworkAddress::new(
                    NetworkPrefix::Mainnet,
                    &Address::P2Pk(private_key.public_image()),
                )
                .to_base58(),
                fee: *BoxValue::SAFE_USER_MIN.as_u64(),
            },
        )
        .unwrap();
        wallet_boxes.push(note.ergo_box().clone());
        let tx_context = TransactionContext::new(
            note_response.transaction,
            wallet_boxes,
            vec![reserve.ergo_box().clone()],
        )
        .unwrap();
        let wallet = Wallet::from_secrets(vec![private_key.into()]);
        let transaction = wallet
            .sign_transaction(tx_context, &force_any_val(), None)
            .unwrap();
        let note_output_tokens = transaction.outputs.get(0).unwrap().tokens.as_ref().unwrap();
        let change_output_tokens = transaction.outputs.get(1).unwrap().tokens.as_ref().unwrap();
        assert_eq!(*(note_output_tokens.get(0).unwrap().amount.as_u64()), 8);
        assert_eq!(*(change_output_tokens.get(0).unwrap().amount.as_u64()), 2);
    }
}
