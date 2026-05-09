//! Creature subclass.

pub mod animal;
pub mod mob;
pub mod squid;

use super::{Living, EntityKind};

pub use animal::Animal;
pub use mob::Mob;
pub use squid::Squid;


crate::class::class! {
    /// Represent a living entity.
    pub struct Creature: Living {
        ..{ Animal, Mob, Squid }
    }
}

impl Creature {

    pub fn kind(&self) -> EntityKind {
        match self.downcast_ref() {
            CreatureRef::None(_) => EntityKind::Internal,
            CreatureRef::Mob(mob) => mob.kind(),
            CreatureRef::Animal(animal) => animal.kind(),
            CreatureRef::Squid(_) => EntityKind::Squid,
        }
    }

}
