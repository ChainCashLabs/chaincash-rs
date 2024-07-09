use std::borrow::BorrowMut;

use diesel::{
    associations::Associations, deserialize::Queryable, prelude::Insertable, Connection,
    RunQueryDsl, Selectable, SelectableHelper,
};

use crate::{
    ergo_boxes::{ErgoBox, ErgoBoxRepository},
    schema, ConnectionPool, ConnectionType, Error,
};

#[derive(Queryable, Selectable, Associations)]
#[diesel(belongs_to(ErgoBox, foreign_key = box_id))]
#[diesel(table_name = schema::notes)]
pub struct Note {
    pub id: i32,
    box_id: i32,
    pub denomination_id: i32,
    pub identifier: String,
    pub value: i64,
    pub owner: String,
}

#[derive(Insertable)]
#[diesel(table_name = schema::notes)]
struct NewNote<'a> {
    identifier: &'a str,
    box_id: i32,
    denomination_id: i32,
    value: i64,
    owner: &'a str,
}

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::ownership_entries)]
#[diesel(belongs_to(Note, foreign_key = note_id))]
struct OwnershipEntry {
    id: i32,
    note_id: i32,
    amount: u64,
    position: u64,
    reserve_nft_id: String,
    signature: Vec<u8>,
}

#[derive(Insertable)]
#[diesel(table_name = schema::ownership_entries)]
#[diesel(belongs_to(Note, foreign_key = note_id))]
struct NewOwnershipEntry {
    note_id: i32,
    amount: i64,
    position: i64,
    reserve_nft_id: String,
    signature: Vec<u8>,
}

pub struct NoteRepository {
    pool: ConnectionPool,
}

impl NoteRepository {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
    fn add_history(
        &self,
        conn: &mut ConnectionType,
        note: &Note,
        note_history: &chaincash_offchain::note_history::NoteHistory,
    ) -> Result<(), Error> {
        let ownership_entries: Vec<NewOwnershipEntry> = note_history
            .ownership_entries()
            .iter()
            .map(|ownership_entry| NewOwnershipEntry {
                note_id: note.id,
                amount: ownership_entry.amount as i64,
                position: ownership_entry.position as i64,
                reserve_nft_id: ownership_entry.reserve_id.into(),
                signature: ownership_entry.signature.serialize(),
            })
            .collect();
        diesel::insert_into(schema::ownership_entries::table)
            .values(ownership_entries)
            .execute(conn)?;
        Ok(())
    }
    pub fn add_note(&self, note: &chaincash_offchain::boxes::Note) -> Result<Note, Error> {
        let mut conn = self.pool.get()?;
        conn.transaction(|conn| {
            let ergo_box = note.ergo_box();
            let created_box = ErgoBoxRepository::add_with_conn(conn.borrow_mut(), ergo_box)?;
            let new_note = NewNote {
                identifier: &String::from(note.note_id),
                box_id: created_box.id,
                denomination_id: 0, // TODO
                value: note.amount.into(),
                owner: &note.owner.to_string(),
            };
            let inserted_note = diesel::insert_into(schema::notes::table)
                .values(&new_note)
                .returning(Note::as_returning())
                .get_result(conn.borrow_mut())?;
            self.add_history(conn.borrow_mut(), &inserted_note, &note.history)?;
            Ok(inserted_note)
        })
    }
}
