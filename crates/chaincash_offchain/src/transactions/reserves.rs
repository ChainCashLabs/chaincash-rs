use crate::boxes::ReserveBoxSpec;

use super::{TransactionError, TxContext};
use ergo_lib::chain::ergo_box::box_builder::ErgoBoxCandidateBuilder;
use ergo_lib::chain::transaction::ergo_transaction::ErgoTransaction;
use ergo_lib::chain::transaction::unsigned::UnsignedTransaction;
use ergo_lib::chain::transaction::Transaction;
use ergo_lib::ergo_chain_types::EcPoint;
use ergo_lib::ergotree_interpreter::sigma_protocol::prover::ContextExtension;
use ergo_lib::ergotree_ir::chain::address::NetworkAddress;
use ergo_lib::ergotree_ir::chain::ergo_box::box_value::BoxValue;
use ergo_lib::ergotree_ir::chain::ergo_box::{ErgoBox, ErgoBoxCandidate};
use ergo_lib::ergotree_ir::chain::token::TokenAmount;
use ergo_lib::ergotree_ir::chain::{ergo_box::NonMandatoryRegisterId, token::Token};
use ergo_lib::ergotree_ir::ergo_tree::ErgoTree;
use ergo_lib::wallet::box_selector::{BoxSelector, SimpleBoxSelector};
use ergo_lib::wallet::{box_selector::BoxSelection, tx_builder::TxBuilder};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct MintReserveRequest {
    pub public_key_hex: String,
    pub amount: u64,
}

pub struct ReserveResponse<T: ErgoTransaction> {
    /// Reserve Box
    pub reserve_box: ReserveBoxSpec,
    /// Unsigned transaction that creates reserve box and mints reserve NFT
    pub transaction: T,
}

pub type SignedReserveResponse = ReserveResponse<Transaction>;

pub fn mint_reserve_transaction(
    request: MintReserveRequest,
    reserve_tree: ErgoTree,
    inputs: BoxSelection<ErgoBox>,
    context: TxContext,
) -> Result<ReserveResponse<UnsignedTransaction>, TransactionError> {
    let pk = EcPoint::try_from(request.public_key_hex).map_err(TransactionError::Parsing)?;
    let mut reserve_box_builder = ErgoBoxCandidateBuilder::new(
        request.amount.try_into()?,
        reserve_tree,
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
    reserve_box_builder.set_register_value(NonMandatoryRegisterId::R4, pk.into());

    let unsigned_transaction = TxBuilder::new(
        inputs,
        vec![reserve_box_builder.build()?],
        context.current_height,
        context.fee.try_into()?,
        NetworkAddress::try_from(context.change_address)?.address(),
    )
    .build()?;

    Ok(ReserveResponse {
        // Unwrap is safe here since transaction layout is fixed (reserve box at output #0)
        reserve_box: unsigned_transaction
            .outputs()
            .first()
            .unwrap()
            .try_into()
            .unwrap(),
        transaction: unsigned_transaction,
    })
}

pub fn top_up_reserve_transaction(
    reserve: &ReserveBoxSpec,
    mut wallet_boxes: Vec<ErgoBox>,
    top_up_amount: u64,
    context: &TxContext,
) -> Result<ReserveResponse<UnsignedTransaction>, TransactionError> {
    if top_up_amount < 1_000_000_000 {
        return Err(TransactionError::TopUpAmountError(top_up_amount));
    }
    wallet_boxes.push(reserve.ergo_box().clone());
    let box_selector = SimpleBoxSelector::new();
    let box_selection = box_selector.select(
        wallet_boxes,
        (reserve.ergo_box().value.as_u64() + top_up_amount + context.fee).try_into()?,
        &[Token {
            token_id: reserve.identifier,
            amount: TokenAmount::try_from(1).unwrap(),
        }],
    )?;
    let mut reserve_box_candidate: ErgoBoxCandidate = reserve.ergo_box().clone().into();
    reserve_box_candidate.value = reserve_box_candidate
        .value
        .checked_add(&top_up_amount.try_into()?)?;
    reserve_box_candidate.creation_height = context.current_height;
    let output_candidates = vec![reserve_box_candidate];
    let mut tx_builder = TxBuilder::new(
        box_selection,
        output_candidates,
        context.current_height,
        context.fee.try_into()?,
        NetworkAddress::try_from(context.change_address.clone())?.address(),
    );
    let mut context_extension = ContextExtension::empty();
    // 10 for top-up action. 1 = top-up, 0 = output index of new reserve box
    context_extension.values.insert(0u8, 10i8.into());
    tx_builder.set_context_extension(reserve.box_id(), context_extension);
    let transaction = tx_builder.build()?;
    let reserve_box = ReserveBoxSpec::try_from(transaction.outputs().first().unwrap()).unwrap();
    Ok(ReserveResponse {
        reserve_box,
        transaction,
    })
}

/// Number of blocks the reserve owner has to wait between announcing a refund and being allowed
/// to withdraw. Mirrors `refundNotificationPeriod` in the `complete refund` action of the reserve
/// contract; the two must be kept in sync.
pub const REFUND_NOTIFICATION_PERIOD: u32 = 14400;

/// Height at which a refund initiated at `refund_height` may be completed.
pub fn refund_unlock_height(refund_height: i32) -> u32 {
    refund_height.max(0) as u32 + REFUND_NOTIFICATION_PERIOD
}

/// Reserve box candidate for a refund action.
///
/// The box is rebuilt from scratch rather than cloned from the input so that the pending-refund
/// registers can be *dropped*: `cancel refund` requires the output to have no R5 at all, and Ergo
/// forbids gaps between non-mandatory registers, so R6 has to go together with R5.
///
/// Everything the contract's `selfPreserved` check looks at - script, tokens and R4 - is copied
/// over unchanged.
fn reserve_refund_candidate(
    reserve: &ReserveBoxSpec,
    value: u64,
    pending_refund: Option<(i32, i64)>,
    current_height: u32,
) -> Result<ErgoBoxCandidate, TransactionError> {
    let mut builder = ErgoBoxCandidateBuilder::new(
        value.try_into()?,
        reserve.ergo_box().ergo_tree.clone(),
        current_height,
    );
    for token in reserve.ergo_box().tokens.as_ref().into_iter().flatten() {
        builder.add_token(token.clone());
    }
    builder.set_register_value(NonMandatoryRegisterId::R4, reserve.owner.clone().into());
    if let Some((refund_height, refund_amount)) = pending_refund {
        builder.set_register_value(NonMandatoryRegisterId::R5, refund_height.into());
        builder.set_register_value(NonMandatoryRegisterId::R6, refund_amount.into());
    }
    Ok(builder.build()?)
}

/// Spend the reserve box back into itself with the given refund `action`, leaving
/// `reserve_out_value` nanoERG in it. Anything the reserve no longer holds goes to the change
/// address, which is where a completed refund is actually paid out.
fn refund_transaction(
    reserve: &ReserveBoxSpec,
    mut wallet_boxes: Vec<ErgoBox>,
    action: i8,
    reserve_out_value: u64,
    pending_refund: Option<(i32, i64)>,
    context: &TxContext,
) -> Result<ReserveResponse<UnsignedTransaction>, TransactionError> {
    wallet_boxes.push(reserve.ergo_box().clone());
    let box_selection = SimpleBoxSelector::new().select(
        wallet_boxes,
        (reserve_out_value + context.fee).try_into()?,
        &[Token {
            token_id: reserve.identifier,
            amount: TokenAmount::try_from(1).unwrap(),
        }],
    )?;
    let reserve_box_candidate = reserve_refund_candidate(
        reserve,
        reserve_out_value,
        pending_refund,
        context.current_height,
    )?;
    let mut tx_builder = TxBuilder::new(
        box_selection,
        vec![reserve_box_candidate],
        context.current_height,
        context.fee.try_into()?,
        NetworkAddress::try_from(context.change_address.clone())?.address(),
    );
    let mut context_extension = ContextExtension::empty();
    // The contract reads a single byte as `action * 10 + index`, where index is the position of
    // the reserve box among the outputs - it is built first here, so 0.
    context_extension.values.insert(0u8, (action * 10).into());
    tx_builder.set_context_extension(reserve.box_id(), context_extension);
    let transaction = tx_builder.build()?;
    // Unwrap is safe here since transaction layout is fixed (reserve box at output #0)
    let reserve_box = ReserveBoxSpec::try_from(transaction.outputs().first().unwrap())?;
    Ok(ReserveResponse {
        reserve_box,
        transaction,
    })
}

/// Announce the intent to withdraw up to `refund_amount` nanoERG from the reserve.
///
/// This does not move any funds: it only writes the current height into R5 and the announced
/// amount into R6, which starts the [`REFUND_NOTIFICATION_PERIOD`] countdown. Note holders keep
/// being able to redeem against the reserve for the whole waiting period.
///
/// The contract accepts an initiation height down to `HEIGHT - 5`, so the transaction has to be
/// included within five blocks of `context.current_height`.
pub fn init_refund_transaction(
    reserve: &ReserveBoxSpec,
    wallet_boxes: Vec<ErgoBox>,
    refund_amount: u64,
    context: &TxContext,
) -> Result<ReserveResponse<UnsignedTransaction>, TransactionError> {
    if reserve.refund_pending() {
        return Err(TransactionError::RefundAlreadyInitiated(reserve.identifier));
    }
    let reserve_value = *reserve.ergo_box().value.as_u64();
    // The reserve box survives a refund, so it has to keep enough to stay a valid box.
    let max_refundable = reserve_value.saturating_sub(*BoxValue::SAFE_USER_MIN.as_u64());
    if refund_amount == 0 || refund_amount > max_refundable {
        return Err(TransactionError::RefundAmountError {
            requested: refund_amount,
            reserve_value,
        });
    }
    refund_transaction(
        reserve,
        wallet_boxes,
        2,
        reserve_value,
        Some((context.current_height as i32, refund_amount as i64)),
        context,
    )
}

/// Call off a pending refund, clearing R5 and R6 and leaving the reserve untouched otherwise.
pub fn cancel_refund_transaction(
    reserve: &ReserveBoxSpec,
    wallet_boxes: Vec<ErgoBox>,
    context: &TxContext,
) -> Result<ReserveResponse<UnsignedTransaction>, TransactionError> {
    if !reserve.refund_pending() {
        return Err(TransactionError::RefundNotInitiated(reserve.identifier));
    }
    refund_transaction(
        reserve,
        wallet_boxes,
        3,
        *reserve.ergo_box().value.as_u64(),
        None,
        context,
    )
}

/// Withdraw an announced refund once the waiting period is over.
///
/// The amount taken out is the announced one, capped at what the reserve can give up while
/// remaining a valid box - a redemption may well have drained it in the meantime, and the
/// contract only ever treats R6 as an upper bound. The withdrawn funds land on the change
/// address, and R5/R6 are cleared so that a further refund needs a fresh announcement.
pub fn complete_refund_transaction(
    reserve: &ReserveBoxSpec,
    wallet_boxes: Vec<ErgoBox>,
    context: &TxContext,
) -> Result<ReserveResponse<UnsignedTransaction>, TransactionError> {
    let (Some(refund_height), Some(refund_amount)) = (reserve.refund_height, reserve.refund_amount)
    else {
        return Err(TransactionError::RefundNotInitiated(reserve.identifier));
    };
    let unlock_height = refund_unlock_height(refund_height);
    if context.current_height < unlock_height {
        return Err(TransactionError::RefundLocked {
            reserve: reserve.identifier,
            unlock_height,
            current_height: context.current_height,
        });
    }
    let reserve_value = *reserve.ergo_box().value.as_u64();
    let max_refundable = reserve_value.saturating_sub(*BoxValue::SAFE_USER_MIN.as_u64());
    let withdrawn = (refund_amount.max(0) as u64).min(max_refundable);
    if withdrawn == 0 {
        return Err(TransactionError::RefundAmountError {
            requested: refund_amount.max(0) as u64,
            reserve_value,
        });
    }
    refund_transaction(
        reserve,
        wallet_boxes,
        4,
        reserve_value - withdrawn,
        None,
        context,
    )
}

#[cfg(test)]
mod test {
    use ergo_lib::{
        chain::{
            ergo_state_context::ErgoStateContext,
            transaction::{unsigned::UnsignedTransaction, Transaction},
        },
        ergotree_interpreter::sigma_protocol::private_input::DlogProverInput,
        ergotree_ir::chain::{
            address::{Address, NetworkAddress, NetworkPrefix},
            ergo_box::{box_value::BoxValue, ErgoBox, NonMandatoryRegisterId},
            token::TokenId,
        },
        wallet::{signing::TransactionContext, Wallet},
    };

    use crate::{
        boxes::ReserveBoxSpec,
        test_util::{create_reserve, create_reserve_with_refund, create_wallet_box, force_any_val},
        transactions::{TransactionError, TxContext},
    };

    use super::{
        cancel_refund_transaction, complete_refund_transaction, init_refund_transaction,
        top_up_reserve_transaction, REFUND_NOTIFICATION_PERIOD,
    };

    #[test]
    fn test_topup() {
        let top_up_amount = 1_000_000_000;
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve(*public_key.clone(), 1_000_000_000);
        let context = TxContext {
            current_height: 0,
            change_address: NetworkAddress::new(
                NetworkPrefix::Mainnet,
                &Address::P2Pk(private_key.public_image()),
            )
            .to_base58(),
            fee: *BoxValue::SAFE_USER_MIN.as_u64(),
        };
        let mut wallet_boxes = vec![create_wallet_box(
            *public_key.clone(),
            top_up_amount + context.fee,
        )];
        let reserve_response =
            top_up_reserve_transaction(&reserve, wallet_boxes.clone(), top_up_amount, &context)
                .unwrap();
        wallet_boxes.push(reserve.ergo_box().clone());
        let tx_context =
            TransactionContext::new(reserve_response.transaction, wallet_boxes, vec![]).unwrap();
        let wallet = Wallet::from_secrets(vec![private_key.into()]);
        let transaction = wallet
            .sign_transaction(tx_context, &force_any_val(), None)
            .unwrap();
        let reserve_output = transaction.outputs.first();
        assert_eq!(
            *reserve_output.value.as_u64(),
            reserve.ergo_box().value.as_u64() + top_up_amount
        );
    }

    const RESERVE_VALUE: u64 = 10_000_000_000;

    /// A blockchain state whose `HEIGHT` - what the contract sees - is `height`.
    fn state_context_at(height: u32) -> ErgoStateContext {
        let mut state_context = force_any_val::<ErgoStateContext>();
        state_context.pre_header.height = height;
        state_context
    }

    fn tx_context(private_key: &DlogProverInput, current_height: u32) -> TxContext {
        TxContext {
            current_height,
            change_address: NetworkAddress::new(
                NetworkPrefix::Mainnet,
                &Address::P2Pk(private_key.public_image()),
            )
            .to_base58(),
            fee: *BoxValue::SAFE_USER_MIN.as_u64(),
        }
    }

    /// Sign `transaction` against the real reserve contract. Signing runs the interpreter over
    /// every input, so a transaction the contract rejects fails here.
    fn sign(
        transaction: UnsignedTransaction,
        reserve: &ReserveBoxSpec,
        wallet_boxes: Vec<ErgoBox>,
        private_key: DlogProverInput,
        state_context: &ErgoStateContext,
    ) -> Transaction {
        let mut inputs = wallet_boxes;
        inputs.push(reserve.ergo_box().clone());
        let tx_context = TransactionContext::new(transaction, inputs, vec![]).unwrap();
        Wallet::from_secrets(vec![private_key.into()])
            .sign_transaction(tx_context, state_context, None)
            .unwrap()
    }

    fn register<T>(ergo_box: &ErgoBox, id: NonMandatoryRegisterId) -> Option<T>
    where
        T: ergo_lib::ergotree_ir::mir::constant::TryExtractFrom<
            ergo_lib::ergotree_ir::mir::constant::Literal,
        >,
    {
        use ergo_lib::ergotree_ir::mir::constant::TryExtractInto;
        ergo_box
            .get_register(id.into())
            .unwrap()
            .map(|reg| reg.v.try_extract_into::<T>().unwrap())
    }

    #[test]
    fn test_init_refund() {
        let refund_amount = 5_000_000_000;
        let height = 1_000_000;
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve(*public_key.clone(), RESERVE_VALUE);
        let context = tx_context(&private_key, height);
        let wallet_boxes = vec![create_wallet_box(*public_key.clone(), context.fee * 2)];

        let response =
            init_refund_transaction(&reserve, wallet_boxes.clone(), refund_amount, &context)
                .unwrap();
        let transaction = sign(
            response.transaction,
            &reserve,
            wallet_boxes,
            private_key,
            &state_context_at(height),
        );

        let reserve_output = transaction.outputs.first();
        // Announcing a refund must not move any funds out of the reserve.
        assert_eq!(*reserve_output.value.as_u64(), RESERVE_VALUE);
        assert_eq!(
            register::<i32>(reserve_output, NonMandatoryRegisterId::R5),
            Some(height as i32)
        );
        assert_eq!(
            register::<i64>(reserve_output, NonMandatoryRegisterId::R6),
            Some(refund_amount as i64)
        );
    }

    #[test]
    fn test_init_refund_rejects_bad_amounts() {
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve(*public_key.clone(), RESERVE_VALUE);
        let context = tx_context(&private_key, 1_000_000);
        let wallet_boxes = vec![create_wallet_box(*public_key.clone(), context.fee * 2)];

        for amount in [0, RESERVE_VALUE, RESERVE_VALUE + 1] {
            assert!(matches!(
                init_refund_transaction(&reserve, wallet_boxes.clone(), amount, &context),
                Err(TransactionError::RefundAmountError { .. })
            ));
        }

        // A reserve that already announced a refund cannot restart the countdown.
        let pending = create_reserve_with_refund(
            *public_key.clone(),
            RESERVE_VALUE,
            Some((999_000, 5_000_000_000)),
        );
        assert!(matches!(
            init_refund_transaction(&pending, wallet_boxes, 1_000_000_000, &context),
            Err(TransactionError::RefundAlreadyInitiated(_))
        ));
    }

    #[test]
    fn test_cancel_refund() {
        let height = 1_000_000;
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve_with_refund(
            *public_key.clone(),
            RESERVE_VALUE,
            Some((height as i32 - 100, 5_000_000_000)),
        );
        let context = tx_context(&private_key, height);
        let wallet_boxes = vec![create_wallet_box(*public_key.clone(), context.fee * 2)];

        let response = cancel_refund_transaction(&reserve, wallet_boxes.clone(), &context).unwrap();
        let transaction = sign(
            response.transaction,
            &reserve,
            wallet_boxes,
            private_key,
            &state_context_at(height),
        );

        let reserve_output = transaction.outputs.first();
        assert_eq!(*reserve_output.value.as_u64(), RESERVE_VALUE);
        // The contract requires R5 to be gone; R6 has to go with it, registers cannot have gaps.
        assert_eq!(
            register::<i32>(reserve_output, NonMandatoryRegisterId::R5),
            None
        );
        assert_eq!(
            register::<i64>(reserve_output, NonMandatoryRegisterId::R6),
            None
        );
    }

    #[test]
    fn test_cancel_refund_without_pending_refund() {
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve(*public_key.clone(), RESERVE_VALUE);
        let context = tx_context(&private_key, 1_000_000);
        assert!(matches!(
            cancel_refund_transaction(&reserve, vec![], &context),
            Err(TransactionError::RefundNotInitiated(_))
        ));
    }

    #[test]
    fn test_complete_refund() {
        let refund_amount = 5_000_000_000;
        let init_height = 1_000_000;
        let height = init_height + REFUND_NOTIFICATION_PERIOD;
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve_with_refund(
            *public_key.clone(),
            RESERVE_VALUE,
            Some((init_height as i32, refund_amount as i64)),
        );
        let context = tx_context(&private_key, height);
        let wallet_boxes = vec![create_wallet_box(*public_key.clone(), context.fee * 2)];

        let response =
            complete_refund_transaction(&reserve, wallet_boxes.clone(), &context).unwrap();
        let transaction = sign(
            response.transaction,
            &reserve,
            wallet_boxes,
            private_key,
            &state_context_at(height),
        );

        let reserve_output = transaction.outputs.first();
        assert_eq!(
            *reserve_output.value.as_u64(),
            RESERVE_VALUE - refund_amount
        );
        // The reserve NFT stays put, only ERG leaves.
        assert_eq!(
            reserve_output.tokens.as_ref().unwrap().first().token_id,
            reserve.identifier
        );
        assert_eq!(
            register::<i32>(reserve_output, NonMandatoryRegisterId::R5),
            None
        );
    }

    #[test]
    fn test_complete_refund_is_locked_until_the_period_is_over() {
        let init_height = 1_000_000;
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve_with_refund(
            *public_key.clone(),
            RESERVE_VALUE,
            Some((init_height as i32, 5_000_000_000)),
        );
        let wallet_boxes = vec![create_wallet_box(
            *public_key.clone(),
            *BoxValue::SAFE_USER_MIN.as_u64() * 2,
        )];

        // One block short of the deadline.
        let height = init_height + REFUND_NOTIFICATION_PERIOD - 1;
        let context = tx_context(&private_key, height);
        assert!(matches!(
            complete_refund_transaction(&reserve, wallet_boxes.clone(), &context),
            Err(TransactionError::RefundLocked { .. })
        ));

        // ...and the contract enforces the same thing, not just the offchain check: build the
        // transaction as if the deadline had passed, then have the chain be one block behind.
        let unlocked_context = tx_context(&private_key, init_height + REFUND_NOTIFICATION_PERIOD);
        let response =
            complete_refund_transaction(&reserve, wallet_boxes.clone(), &unlocked_context).unwrap();
        let mut inputs = wallet_boxes;
        inputs.push(reserve.ergo_box().clone());
        let tx_context = TransactionContext::new(response.transaction, inputs, vec![]).unwrap();
        assert!(Wallet::from_secrets(vec![private_key.into()])
            .sign_transaction(tx_context, &state_context_at(height), None)
            .is_err());
    }

    #[test]
    fn test_complete_refund_caps_withdrawal_at_what_the_reserve_still_holds() {
        // A redemption may have drained the reserve after the refund was announced. R6 is only an
        // upper bound for the contract, so the refund should still go through for what is left.
        let init_height = 1_000_000;
        let height = init_height + REFUND_NOTIFICATION_PERIOD;
        let announced = 5_000_000_000;
        let remaining = 2_000_000_000;
        let private_key = DlogProverInput::random();
        let public_key = private_key.public_image().h;
        let reserve = create_reserve_with_refund(
            *public_key.clone(),
            remaining,
            Some((init_height as i32, announced)),
        );
        let context = tx_context(&private_key, height);
        let wallet_boxes = vec![create_wallet_box(*public_key.clone(), context.fee * 2)];

        let response =
            complete_refund_transaction(&reserve, wallet_boxes.clone(), &context).unwrap();
        let transaction = sign(
            response.transaction,
            &reserve,
            wallet_boxes,
            private_key,
            &state_context_at(height),
        );

        assert_eq!(
            *transaction.outputs.first().value.as_u64(),
            *BoxValue::SAFE_USER_MIN.as_u64()
        );
    }

    #[test]
    fn test_reserve_spec_reads_pending_refund_registers() {
        // Regression test: R5 is declared `Int` and R6 `Long` by the reserve contract. Reading R5
        // as a `Long` made every reserve with a pending refund fail to parse, which silently
        // dropped it from the scanner and from the reserve list.
        let public_key = DlogProverInput::random().public_image().h;
        let reserve =
            create_reserve_with_refund(*public_key.clone(), RESERVE_VALUE, Some((123_456, 789)));
        assert_eq!(reserve.refund_height, Some(123_456));
        assert_eq!(reserve.refund_amount, Some(789));
        assert!(reserve.refund_pending());

        let reserve = ReserveBoxSpec::try_from(reserve.ergo_box()).unwrap();
        assert_eq!(reserve.refund_height, Some(123_456));
        assert_eq!(reserve.refund_amount, Some(789));

        let no_refund = create_reserve(*public_key, RESERVE_VALUE);
        assert_eq!(no_refund.refund_height, None);
        assert_eq!(no_refund.refund_amount, None);
        assert!(!no_refund.refund_pending());
        let _: TokenId = no_refund.identifier;
    }
}
