//! Chest block entity.

use crate::item::ItemStack;


#[derive(Debug, Clone, Default)]
pub struct Chest {
    /// The inventory of the chest.
    pub inv: Box<[ItemStack; 27]>,
}
