use crate::ergo_boxes::ErgoBoxRepository;
use crate::schema;
use crate::ConnectionPool;
use crate::Error;
use chaincash_specs::boxes::ReserveBoxSpec;
use diesel::prelude::*;
use ergo_lib::ergotree_ir::chain::ergo_box::ErgoBox;
use std::borrow::BorrowMut;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::reserves)]
pub struct Reserve {
    pub id: i32,
    pub box_id: i32,
    pub owner: String,
}

#[derive(Insertable)]
#[diesel(table_name = schema::reserves)]
pub struct NewReserve<'a> {
    pub box_id: i32,
    pub owner: &'a str,
}

pub struct ReserveRepository {
    pool: ConnectionPool,
}

impl ReserveRepository {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    pub fn create(&self, ergo_box: &ErgoBox) -> Result<Reserve, Error> {
        let reserve_spec = ReserveBoxSpec::try_from(ergo_box)?;
        let mut conn = self.pool.get()?;
        let created_box =
            ErgoBoxRepository::create_with_conn(conn.borrow_mut(), ergo_box.try_into()?)?;
        let new_reserve = NewReserve {
            box_id: created_box.id,
            owner: &reserve_spec.owner.to_string(),
        };
        Ok(diesel::insert_into(schema::reserves::table)
            .values(&new_reserve)
            .returning(Reserve::as_returning())
            .get_result(conn.borrow_mut())?)
    }
}
