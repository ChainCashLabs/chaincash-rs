use crate::database::ConnectionPool;

#[derive(Clone)]
pub struct NoteService {
    pool: ConnectionPool,
}

impl NoteService {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
