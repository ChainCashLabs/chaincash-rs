use crate::database::ConnectionPool;

pub struct ReserveService {
    pool: ConnectionPool,
}

impl ReserveService {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }
}
