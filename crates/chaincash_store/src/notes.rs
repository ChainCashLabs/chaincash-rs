use crate::ConnectionPool;

pub struct Note {}

pub struct NoteService {
    pool: ConnectionPool,
}

impl NoteService {
    pub(crate) fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
