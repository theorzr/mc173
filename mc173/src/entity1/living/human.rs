use crate::world::World;

use super::Living;


crate::class::class! {
    /// The human entity.
    pub struct Human: Living {
        pub username: String,
    }
}

impl Human {

    pub fn tick(&mut self, world: &mut World, id: u32) {
        
    }

}
