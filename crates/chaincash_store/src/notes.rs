pub struct Note {}

pub trait NoteService: Send + Sync {
    fn create(&self) -> Note;
}
