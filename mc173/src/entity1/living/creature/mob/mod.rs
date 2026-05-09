pub mod creeper;
pub mod giant;
pub mod skeleton;
pub mod spider;
pub mod zombie;
pub mod pig_zombie;

use super::{Creature, EntityKind};

pub use creeper::Creeper;
pub use giant::Giant;
pub use skeleton::Skeleton;
pub use spider::Spider;
pub use zombie::Zombie;
pub use pig_zombie::PigZombie;


crate::class::class! {
    pub struct Mob: Creature {
        ..{ Creeper, Giant, Skeleton, Spider, Zombie, PigZombie }
    }
}

impl Mob {

    pub fn kind(&self) -> EntityKind {
        match self.downcast_ref() {
            MobRef::None(_) => EntityKind::Internal,
            MobRef::Creeper(_) => EntityKind::Creeper,
            MobRef::Giant(_) => EntityKind::Giant,
            MobRef::Skeleton(_) => EntityKind::Skeleton,
            MobRef::Spider(_) => EntityKind::Spider,
            MobRef::Zombie(_) => EntityKind::Zombie,
            MobRef::PigZombie(_) => EntityKind::PigZombie,
        }
    }

}
