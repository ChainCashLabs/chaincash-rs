use crate::ergo_boxes::ErgoBoxRepository;
use crate::schema;
use crate::ConnectionPool;
use crate::Error;
use chaincash_offchain::boxes::ReserveBoxSpec;
use diesel::prelude::*;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use std::borrow::BorrowMut;

#[derive(Queryable, Selectable, Associations)]
#[diesel(belongs_to(ErgoBox, foreign_key = box_id))]
#[diesel(table_name = schema::reserves)]
pub struct Reserve {
    pub id: i32,
    pub box_id: i32,
    pub denomination_id: i32,
    /// NFT ID that uniquely identifies this reserve.
    pub identifier: String,
    /// Owner of the reserve, GE encoded as hex string.
    pub owner: String,
}

#[derive(Insertable)]
#[diesel(table_name = schema::reserves)]
pub struct NewReserve<'a> {
    pub box_id: i32,
    pub denomination_id: i32,
    pub identifier: &'a str,
    pub owner: &'a str,
}

pub struct ReserveRepository {
    pool: ConnectionPool,
}

impl ReserveRepository {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    pub fn add(&self, reserve_box: &ReserveBoxSpec) -> Result<Reserve, Error> {
        let ergo_box = reserve_box.ergo_box();
        let mut conn = self.pool.get()?;
        let created_box = ErgoBoxRepository::add_with_conn(conn.borrow_mut(), ergo_box)?;
        let new_reserve = NewReserve {
            box_id: created_box.id,
            denomination_id: 0, // TODO, allow setting different denominations, should be auto detected by inspecting the ErgoBox
            owner: &reserve_box.owner.to_string(),
            identifier: &reserve_box.identifier,
        };
        Ok(diesel::insert_into(schema::reserves::table)
            .values(&new_reserve)
            .returning(Reserve::as_returning())
            .get_result(conn.borrow_mut())?)
    }
    pub fn reserve_boxes(&self) -> Result<Vec<ReserveBoxSpec>, Error> {
        // let mut conn = self.pool.get()?;
        // schema::reserves::table
        //     .inner_join(schema::ergo_boxes::table)
        //     .load::<(Reserve, ErgoBox)>(&mut conn)
        //     .map_err(|e| e.into());
        todo!()
    }
}
