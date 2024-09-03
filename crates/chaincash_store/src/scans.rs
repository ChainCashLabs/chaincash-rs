//! Scans for Receipts & Reserve Contracts. Note contracts are handled seperately

use std::borrow::{BorrowMut, Cow};

use diesel::{
    prelude::{AsChangeset, Insertable, Queryable},
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, Selectable,
};

use crate::{schema, ConnectionPool, Error};

#[derive(Queryable, Selectable, Insertable, AsChangeset, Debug)]
#[diesel(table_name = schema::scans)]
pub struct Scan<'a> {
    pub scan_id: i32,
    pub scan_type: Cow<'a, str>,
    pub scan_name: Cow<'a, str>,
}

impl<'a> Scan<'a> {
    pub fn new(scan_id: u32, scan_name: impl Into<Cow<'a, str>>, scan_type: ScanType) -> Scan<'a> {
        Scan {
            scan_id: scan_id as i32,
            scan_name: scan_name.into(),
            scan_type: scan_type.to_str().into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ScanType {
    Reserves,
    Receipts,
    Notes,
}

impl ScanType {
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Reserves => "reserve",
            Self::Receipts => "receipt",
            Self::Notes => "note",
        }
    }
}

pub struct ScanRepository {
    pool: ConnectionPool,
}

impl ScanRepository {
    pub fn new(pool: ConnectionPool) -> Self {
        ScanRepository { pool }
    }
    pub fn add(&self, scan: &Scan) -> Result<(), Error> {
        let mut conn = self.pool.get()?;
        diesel::insert_into(schema::scans::table)
            .values(scan)
            .returning(schema::scans::scan_id)
            .execute(conn.borrow_mut())?;
        Ok(())
    }
    pub fn delete(&self, scan_id: i32) -> Result<(), Error> {
        let mut conn = self.pool.get()?;
        diesel::delete(schema::scans::table)
            .filter(schema::scans::scan_id.eq(scan_id))
            .execute(conn.borrow_mut())?;
        Ok(())
    }
    pub fn scans_by_type(&self, scan_type: ScanType) -> Result<Vec<Scan<'static>>, Error> {
        let mut conn = self.pool.get()?;
        Ok(schema::scans::table
            .filter(schema::scans::scan_type.eq(scan_type.to_str()))
            .load::<Scan<'_>>(conn.borrow_mut())?)
    }
}
