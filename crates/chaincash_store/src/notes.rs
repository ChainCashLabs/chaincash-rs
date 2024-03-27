use crate::ConnectionPool;

pub struct Note {}

pub struct NoteRepository {
    pool: ConnectionPool,
}

impl NoteRepository {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
