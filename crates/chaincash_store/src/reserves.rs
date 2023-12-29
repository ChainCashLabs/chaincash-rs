pub struct Reserve {}

pub trait ReserveService {
    fn create(&self) -> Reserve;
}
