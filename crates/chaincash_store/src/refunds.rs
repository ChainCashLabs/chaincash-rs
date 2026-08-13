//! Refunds announced on reserves.
//!
//! A refund is a two-step, timelocked withdrawal: the reserve owner announces it on chain, waits
//! out [`REFUND_NOTIFICATION_PERIOD`] blocks so note holders can still redeem, and only then takes
//! the funds out. The reserve box itself only remembers the *pending* step - once the refund is
//! settled the registers are cleared and the box is spent - so the history lives here.

use std::borrow::BorrowMut;

use chaincash_offchain::transactions::reserves::refund_unlock_height;
use diesel::prelude::*;
use ergo_lib::ergo_chain_types::Digest32;
use ergo_lib::ergotree_ir::chain::token::TokenId;
use serde::Serialize;

use crate::{schema, ConnectionPool, Error};

/// Where a refund is in its lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RefundStatus {
    /// Announced on chain, waiting for the notification period to pass.
    Initiated,
    /// Funds withdrawn.
    Completed,
    /// Called off by the owner before completion.
    Cancelled,
}

impl RefundStatus {
    pub fn to_str(self) -> &'static str {
        match self {
            Self::Initiated => "initiated",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Read back a status stored in the database. `None` for anything the `status` CHECK
    /// constraint should have kept out in the first place.
    pub fn parse(status: &str) -> Option<Self> {
        match status {
            "initiated" => Some(Self::Initiated),
            "completed" => Some(Self::Completed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = schema::refunds)]
pub struct Refund {
    pub id: i32,
    /// NFT ID of the reserve being refunded.
    pub reserve_identifier: String,
    /// nanoERG announced at initiation.
    pub amount: i64,
    /// nanoERG actually withdrawn, only set once the refund completed.
    pub withdrawn_amount: Option<i64>,
    /// Height the refund was announced at, i.e. what went into R5 of the reserve box.
    pub init_height: i32,
    status: String,
    /// Transaction that announced the refund. `None` for a refund this server did not announce
    /// itself and picked up from the chain.
    pub init_tx_id: Option<String>,
    /// Transaction that completed or cancelled the refund.
    pub settle_tx_id: Option<String>,
}

impl Refund {
    pub fn status(&self) -> Option<RefundStatus> {
        RefundStatus::parse(&self.status)
    }

    /// NFT ID of the reserve being refunded.
    pub fn reserve_id(&self) -> Result<TokenId, Error> {
        Digest32::try_from(self.reserve_identifier.clone())
            .map(TokenId::from)
            .map_err(|_| Error::InvalidTokenId(self.reserve_identifier.clone()))
    }

    /// Height from which the refund may be completed.
    pub fn unlock_height(&self) -> u32 {
        refund_unlock_height(self.init_height)
    }
}

/// A refund as served by the API.
#[derive(Serialize)]
pub struct RefundInfo {
    pub id: i32,
    pub reserve_id: String,
    pub amount: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withdrawn_amount: Option<i64>,
    pub init_height: i32,
    /// Height from which the refund may be completed.
    pub unlock_height: u32,
    pub status: Option<RefundStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_tx_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settle_tx_id: Option<String>,
}

impl From<Refund> for RefundInfo {
    fn from(refund: Refund) -> Self {
        Self {
            id: refund.id,
            unlock_height: refund.unlock_height(),
            status: refund.status(),
            reserve_id: refund.reserve_identifier,
            amount: refund.amount,
            withdrawn_amount: refund.withdrawn_amount,
            init_height: refund.init_height,
            init_tx_id: refund.init_tx_id,
            settle_tx_id: refund.settle_tx_id,
        }
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::refunds)]
struct NewRefund<'a> {
    reserve_identifier: &'a str,
    amount: i64,
    init_height: i32,
    status: &'a str,
    init_tx_id: Option<&'a str>,
}

pub struct RefundRepository {
    pool: ConnectionPool,
}

impl RefundRepository {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    /// Record a refund that has been announced on chain.
    ///
    /// `init_tx_id` is `None` for a refund this server did not announce itself and only found on
    /// the reserve box.
    ///
    /// A reserve can only have one pending refund at a time, enforced by a unique index, so a
    /// second initiation fails here rather than on chain.
    pub fn add(
        &self,
        reserve_id: &TokenId,
        amount: i64,
        init_height: i32,
        init_tx_id: Option<&str>,
    ) -> Result<Refund, Error> {
        let mut conn = self.pool.get()?;
        Ok(diesel::insert_into(schema::refunds::table)
            .values(&NewRefund {
                reserve_identifier: &String::from(*reserve_id),
                amount,
                init_height,
                status: RefundStatus::Initiated.to_str(),
                init_tx_id,
            })
            .returning(Refund::as_returning())
            .get_result(conn.borrow_mut())?)
    }

    /// Mark a refund as completed, recording what was actually withdrawn.
    pub fn complete(
        &self,
        id: i32,
        withdrawn_amount: i64,
        settle_tx_id: &str,
    ) -> Result<Refund, Error> {
        self.settle(
            id,
            RefundStatus::Completed,
            Some(withdrawn_amount),
            Some(settle_tx_id),
        )
    }

    /// Mark a refund as cancelled.
    ///
    /// `settle_tx_id` is `None` when the refund stopped being pending without this server doing
    /// it - the announcement is gone from the reserve box and there is no transaction of ours to
    /// point at.
    pub fn cancel(&self, id: i32, settle_tx_id: Option<&str>) -> Result<Refund, Error> {
        self.settle(id, RefundStatus::Cancelled, None, settle_tx_id)
    }

    /// Settle a refund. The `status` filter makes this a no-op - a `NotFound` error - for a refund
    /// that was already settled, so an outcome is never overwritten.
    fn settle(
        &self,
        id: i32,
        status: RefundStatus,
        withdrawn_amount: Option<i64>,
        settle_tx_id: Option<&str>,
    ) -> Result<Refund, Error> {
        let mut conn = self.pool.get()?;
        Ok(diesel::update(schema::refunds::table)
            .filter(schema::refunds::id.eq(id))
            .filter(schema::refunds::status.eq(RefundStatus::Initiated.to_str()))
            .set((
                schema::refunds::status.eq(status.to_str()),
                schema::refunds::withdrawn_amount.eq(withdrawn_amount),
                schema::refunds::settle_tx_id.eq(settle_tx_id),
            ))
            .returning(Refund::as_returning())
            .get_result(conn.borrow_mut())?)
    }

    /// The pending refund of a reserve, if any.
    pub fn pending_for_reserve(&self, reserve_id: &TokenId) -> Result<Option<Refund>, Error> {
        let mut conn = self.pool.get()?;
        Ok(schema::refunds::table
            .filter(schema::refunds::reserve_identifier.eq(String::from(*reserve_id)))
            .filter(schema::refunds::status.eq(RefundStatus::Initiated.to_str()))
            .select(Refund::as_select())
            .first(conn.borrow_mut())
            .optional()?)
    }

    /// Every refund on record, newest first.
    pub fn all(&self) -> Result<Vec<Refund>, Error> {
        let mut conn = self.pool.get()?;
        Ok(schema::refunds::table
            .order(schema::refunds::id.desc())
            .select(Refund::as_select())
            .load(conn.borrow_mut())?)
    }

    /// Every refund on record for a single reserve, newest first.
    pub fn by_reserve(&self, reserve_id: &TokenId) -> Result<Vec<Refund>, Error> {
        let mut conn = self.pool.get()?;
        Ok(schema::refunds::table
            .filter(schema::refunds::reserve_identifier.eq(String::from(*reserve_id)))
            .order(schema::refunds::id.desc())
            .select(Refund::as_select())
            .load(conn.borrow_mut())?)
    }
}

#[cfg(test)]
mod test {
    use chaincash_offchain::transactions::reserves::REFUND_NOTIFICATION_PERIOD;

    use super::*;
    use crate::{ChainCashStore, Update};

    fn store() -> ChainCashStore {
        let store = ChainCashStore::open_in_memory().unwrap();
        store.update().unwrap();
        store
    }

    fn token_id(hex: &str) -> TokenId {
        TokenId::from(Digest32::try_from(hex.to_owned()).unwrap())
    }

    fn reserve_a() -> TokenId {
        token_id("161A3A5250655368566D597133743677397A24432646294A404D635166546A57")
    }

    fn reserve_b() -> TokenId {
        token_id("4b2d8b7beb3eaac8234d9e61792d270898a43934d6a27275e4f3a044609c9f2a")
    }

    #[test]
    fn test_add_and_complete() {
        let store = store();
        let refunds = store.refunds();
        let refund = refunds
            .add(&reserve_a(), 5_000_000_000, 1_000, Some("txinit"))
            .unwrap();
        assert_eq!(refund.status(), Some(RefundStatus::Initiated));
        assert_eq!(refund.unlock_height(), 1_000 + REFUND_NOTIFICATION_PERIOD);
        assert!(refunds.pending_for_reserve(&reserve_a()).unwrap().is_some());

        let completed = refunds
            .complete(refund.id, 4_000_000_000, "txdone")
            .unwrap();
        assert_eq!(completed.status(), Some(RefundStatus::Completed));
        assert_eq!(completed.withdrawn_amount, Some(4_000_000_000));
        assert_eq!(completed.settle_tx_id.as_deref(), Some("txdone"));
        // Settled refunds stay on record but are no longer pending.
        assert!(refunds.pending_for_reserve(&reserve_a()).unwrap().is_none());
        assert_eq!(refunds.by_reserve(&reserve_a()).unwrap().len(), 1);
    }

    #[test]
    fn test_cancel_frees_the_reserve_for_a_new_refund() {
        let store = store();
        let refunds = store.refunds();
        let refund = refunds
            .add(&reserve_a(), 1_000_000_000, 1_000, Some("txinit"))
            .unwrap();

        // Only one refund may be pending per reserve.
        assert!(refunds
            .add(&reserve_a(), 1, 1_001, Some("txinit2"))
            .is_err());

        refunds.cancel(refund.id, Some("txcancel")).unwrap();
        let second = refunds
            .add(&reserve_a(), 2_000_000_000, 2_000, Some("txinit3"))
            .unwrap();
        assert_eq!(second.status(), Some(RefundStatus::Initiated));
        assert_eq!(refunds.by_reserve(&reserve_a()).unwrap().len(), 2);
        // Newest first.
        assert_eq!(refunds.all().unwrap().first().unwrap().id, second.id);
    }

    #[test]
    fn test_settling_twice_is_rejected() {
        let store = store();
        let refunds = store.refunds();
        let refund = refunds
            .add(&reserve_a(), 1_000_000_000, 1_000, Some("txinit"))
            .unwrap();
        refunds
            .complete(refund.id, 1_000_000_000, "txdone")
            .unwrap();
        assert!(refunds
            .complete(refund.id, 1_000_000_000, "txagain")
            .is_err());
        assert!(refunds.cancel(refund.id, Some("txcancel")).is_err());
    }

    #[test]
    fn test_unlock_height_is_the_waiting_period_past_initiation() {
        let store = store();
        let refunds = store.refunds();
        let refund = refunds
            .add(&reserve_a(), 1_000_000_000, 1_000, None)
            .unwrap();
        assert_eq!(refund.unlock_height(), 1_000 + REFUND_NOTIFICATION_PERIOD);
        // A refund found on chain rather than announced here has no initiating transaction.
        assert_eq!(refund.init_tx_id, None);
        assert_eq!(refund.reserve_id().unwrap(), reserve_a());
    }

    #[test]
    fn test_refunds_are_tracked_per_reserve() {
        let store = store();
        let refunds = store.refunds();
        let a = refunds
            .add(&reserve_a(), 1_000_000_000, 1_000, Some("txa"))
            .unwrap();
        let b = refunds
            .add(&reserve_b(), 2_000_000_000, 5_000, Some("txb"))
            .unwrap();

        assert_eq!(refunds.all().unwrap().len(), 2);
        assert_eq!(
            refunds
                .by_reserve(&reserve_a())
                .unwrap()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
            vec![a.id]
        );
        assert_eq!(
            refunds
                .pending_for_reserve(&reserve_b())
                .unwrap()
                .unwrap()
                .id,
            b.id
        );

        // Settling one reserve's refund leaves the other alone.
        refunds.complete(a.id, 1_000_000_000, "txdone").unwrap();
        assert!(refunds.pending_for_reserve(&reserve_a()).unwrap().is_none());
        assert!(refunds.pending_for_reserve(&reserve_b()).unwrap().is_some());
    }
}
