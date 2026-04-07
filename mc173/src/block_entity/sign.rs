//! Sign block entity.


#[derive(Debug, Clone, Default)]
pub struct Sign {
    /// Text line of this sign block.
    pub lines: Box<[String; 4]>,
}
