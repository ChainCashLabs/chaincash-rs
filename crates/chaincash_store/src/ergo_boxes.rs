use std::borrow::BorrowMut;

use crate::{schema, ConnectionPool, ConnectionType, Error};
use diesel::prelude::*;
use ergo_lib::ergotree_ir::{
    chain::ergo_box::ErgoBox as NetworkBox, serialization::SigmaSerializable,
};

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::ergo_boxes)]
pub struct ErgoBox {
    pub id: i32,
    pub ergo_id: String,
    pub bytes: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::ergo_boxes)]
pub struct NewErgoBox {
    pub ergo_id: String,
    pub bytes: Vec<u8>,
}

impl TryFrom<&NetworkBox> for NewErgoBox {
    type Error = Error;

    fn try_from(value: &NetworkBox) -> Result<Self, Self::Error> {
        Ok(Self {
            ergo_id: value.box_id().to_string(),
            bytes: value.sigma_serialize_bytes().unwrap(),
        })
    }
}

pub struct ErgoBoxRepository {
    pool: ConnectionPool,
}

impl ErgoBoxRepository {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    pub fn add(&self, b: &NetworkBox) -> Result<ErgoBox, Error> {
        Self::add_with_conn(self.pool.get()?.borrow_mut(), b)
    }

    pub(crate) fn add_with_conn(
        conn: &mut ConnectionType,
        b: &NetworkBox,
    ) -> Result<ErgoBox, Error> {
        let new_box = NewErgoBox::try_from(b)?;
        Ok(diesel::insert_into(schema::ergo_boxes::table)
            .values(new_box)
            .returning(ErgoBox::as_returning())
            .get_result(conn)?)
    }
}
