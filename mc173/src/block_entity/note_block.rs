//! Note block block entity.


#[derive(Debug, Clone, Default)]
pub struct NoteBlock {
    /// The note to play.
    pub note: u8,
    /// True if the note block is currently powered.
    pub powered: bool,
}
