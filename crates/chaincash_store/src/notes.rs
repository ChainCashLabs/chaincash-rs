pub struct Note {}

pub trait NoteService {
    fn create(&self) -> Note;
}
