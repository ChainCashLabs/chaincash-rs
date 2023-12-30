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

pub struct ErgoBoxService {
    pool: ConnectionPool,
}

impl ErgoBoxService {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    pub fn create(&self, b: NewErgoBox) -> Result<ErgoBox, Error> {
        Self::create_with_conn(self.pool.get()?.borrow_mut(), b)
    }

    pub(crate) fn create_with_conn(
        conn: &mut ConnectionType,
        b: NewErgoBox,
    ) -> Result<ErgoBox, Error> {
        Ok(diesel::insert_into(schema::ergo_boxes::table)
            .values(&b)
            .returning(ErgoBox::as_returning())
            .get_result(conn)?)
    }
}
