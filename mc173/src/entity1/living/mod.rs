mod human;

use crate::world::World;

use super::{Entity, EntityKind};

pub use human::Human;


crate::class::class! {
    /// Represent a living entity.
    pub struct Living: Entity {
        /// Set to true if an entity is artificial, as opposed to natural. If not artificial,
        /// an entity is despawned when too far from the closest player (maximum distance of 
        /// 128.0 blocks).
        pub artificial: bool,
        /// The health.
        pub health: u16,
        /// This timer is used on entities that are wandering too far from players or that
        /// take hurt damages. This is only used on entities that are AI ticked and on non
        /// persistent living entities. When this time reaches 600 and there are players in
        /// the 128.0 block distance, then this entity has 1/800 chance of despawning.
        pub wander_time: u16,
        /// TBD.
        pub hurt_time: u16,
        /// The last hurt damage taken since `hurt_time` is not zero.
        pub hurt_damage: u16,
        /// On the first damage, this is the yaw of the hurt's origin.
        pub hurt_yaw: f32,
        /// TBD.
        pub attack_time: u16,
        /// The death timer, increasing each tick when no health, after 20 ticks the entity
        /// is definitely removed from the world.
        pub death_time: u16,
        ..{ Human }
    }
}

impl Living {

    pub fn kind(&self) -> EntityKind {
        match self.downcast_ref() {
            LivingRef::None(_) => unreachable!(),
            LivingRef::Human(_) => EntityKind::Human,
        }
    }

    pub fn can_natural_spawn(&self, world: &World) -> bool {
        match self.downcast_ref() {
            LivingRef::Human(human) => todo!(),
            _ => self._can_natural_spawn(world),
        }
    }

    fn _can_natural_spawn(&self, world: &World) -> bool {

        // Real impl:
        // checkIfAABBIsClear(self.bb) &&
        // getCollidingBoundingBoxes(self, self.bb) == 0 &&
        // !getIsAnyLiquid(self.bb)
        
        for (_, entity) in world.iter_entities_colliding(self.bb) {
            if entity.prevent_spawning || entity.hard {
                return false;
            }
        }

        if world.iter_block_boxes_colliding(self.bb).next().is_some() {
            return false;
        }

        true

    }

    pub fn init_natural_spawn(&mut self, world: &mut World, id: u32) {
        match self.downcast_mut() {
            LivingMut::Human(human) => todo!(),
            _ => ()
        }
    }

    pub fn tick(&mut self, world: &mut World, id: u32) {
        match self.downcast_mut() {
            LivingMut::None(_) => self._tick(world, id),
            LivingMut::Human(human) => human.tick(world, id),
        }
    }

    pub fn _tick(&mut self, world: &mut World, id: u32) {
        
    }

}

// impl Entity<Living> {

//     fn tick() {
        
//     }

// }

// pub(super) fn tick_dispatch(entity: &mut Entity<Living>, world: &mut World, id: u32) {
//     match entity.class.downcast_mut() {
//         LivingClassMut::Human(living) => todo!(),
//     }
// }
