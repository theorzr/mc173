pub mod chicken;
pub mod cow;
pub mod pig;
pub mod sheep;
pub mod wolf;

use super::{Creature, EntityKind};

pub use chicken::Chicken;
pub use cow::Cow;
pub use pig::Pig;
pub use sheep::Sheep;
pub use wolf::Wolf;


crate::class::class! {
    pub struct Animal: Creature {
        ..{ Chicken, Cow, Pig, Sheep, Wolf }
    }
}

impl Animal {

    pub fn kind(&self) -> EntityKind {
        match self.downcast_ref() {
            AnimalRef::None(_) => EntityKind::Internal,
            AnimalRef::Chicken(_) => EntityKind::Chicken,
            AnimalRef::Cow(_) => EntityKind::Cow,
            AnimalRef::Pig(_) => EntityKind::Pig,
            AnimalRef::Sheep(_) => EntityKind::Sheep,
            AnimalRef::Wolf(_) => EntityKind::Wolf,
        }
    }

}
