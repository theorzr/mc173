//! Living entity class.

use glam::IVec3;

use crate::world::World;

use super::base::{Base, BaseTick};


/// The data common to all living entities.
#[derive(Debug, Clone, Default)]
pub struct Living {
    /// The base.
    pub base: Base,
    /// Set to true if an entity is artificial, as opposed to natural. If not artificial,
    /// an entity is despawned when too far from the closest player (maximum distance of 
    /// 128.0 blocks).
    pub artificial: bool,
    /// The health.
    pub health: u16,
    /// The last damage inflicted to the entity during `hurt_time`, this is used to only
    /// damage for the maximum damage inflicted while `hurt_time` is not zero.
    pub hurt_last_damage: u16,
    /// Hurt countdown, read `hurt_damage` documentation.
    pub hurt_time: u16,
    /// TBD.
    pub attack_time: u16,
    /// The death timer, increasing each tick when no health, after 20 ticks the entity
    /// is definitely removed from the world.
    pub death_time: u16,
    /// The strafing acceleration.
    pub accel_strafing: f32,
    /// The forward acceleration.
    pub accel_forward: f32,
    /// Velocity of the look's yaw axis.
    pub yaw_velocity: f32,
    /// True if this entity is trying to jump.
    pub jumping: bool,
    /// If this entity is looking at another one.
    pub look_target: Option<LookTarget>,
    /// If this entity is attacking another one.
    pub attack_target: Option<u32>,
    /// The path this creature needs to follow.
    pub path: Option<Path>,
    /// This timer is used on entities that are wandering too far from players or that
    /// take hurt damages. This is only used on entities that are AI ticked and on non
    /// persistent living entities. When this time reaches 600 and there are players in
    /// the 128.0 block distance, then this entity has 1/800 chance of despawning.
    pub wander_time: u16,
}

/// Trait to implement ticking for a living entity.
pub trait LivingTick: BaseTick {

    fn living(&self) -> &Living;
    fn living_mut(&mut self) -> &mut Living;

    fn tick(&mut self, world: &mut World, id: u32) {
        
        BaseTick::tick(self, world, id);
        self.tick_living(world, id);
        todo!()

    }
    
    fn tick_base(&mut self, world: &mut World, id: u32) {
        BaseTick::tick_base(self, world, id);
    }

    fn tick_living(&mut self, world: &mut World, id: u32) {
        todo!()
    }

    fn handle_water(&mut self, world: &mut World, id: u32) -> bool {
        todo!()
    }

    fn handle_lava(&mut self, world: &mut World, id: u32) -> bool {
        todo!()
    }

    fn hurt_from(&mut self, world: &mut World, id: u32, other_id: Option<u32>, damage: u32) -> bool {
        todo!()
    }

    fn hurt_from_void(&mut self, world: &mut World, id: u32) {
        world.remove_entity(id, "kill from void");
    }

}

// Auto implementation to just call the override methods.
impl<T: LivingTick> BaseTick for T {

    #[inline]
    fn base(&self) -> &Base {
        &LivingTick::living(self).base
    }

    #[inline]
    fn base_mut(&mut self) -> &mut Base {
        &mut LivingTick::living_mut(self).base
    }

    #[inline]
    fn tick(&mut self, world: &mut World, id: u32) {
        LivingTick::tick(self, world, id);
    }

    #[inline]
    fn tick_base(&mut self, world: &mut World, id: u32) {
        LivingTick::tick_base(self, world, id);
    }

    #[inline]
    fn handle_water_(&mut self, world: &mut World, id: u32) -> bool {
        LivingTick::handle_water(self, world, id)
    }

    #[inline]
    fn handle_lava(&mut self, world: &mut World, id: u32) -> bool {
        LivingTick::handle_lava(self, world, id)
    }

    #[inline]
    fn hurt_from(&mut self, world: &mut World, id: u32, other_id: Option<u32>, damage: u32) -> bool {
        LivingTick::hurt_from(self, world, id, other_id, damage)
    }

    #[inline]
    fn hurt_from_void(&mut self, world: &mut World, id: u32) {
        LivingTick::hurt_from_void(self, world, id);
    }

}

/// Define a target for an entity to look at.
#[derive(Debug, Clone, Default)]
pub struct LookTarget {
    /// The entity id to look at.
    pub entity_id: u32,
    /// Ticks remaining before stop looking at it.
    pub remaining_time: u32,
}

/// A result of the path finder.
#[derive(Debug, Clone)]
pub struct Path {
    pub points: Vec<IVec3>,
    pub index: usize,
}

impl From<Vec<IVec3>> for Path {
    fn from(points: Vec<IVec3>) -> Self {
        Self { points, index: 0 }
    }
}

impl From<IVec3> for Path {
    fn from(value: IVec3) -> Self {
        Self { points: vec![value], index: 0 }
    }
}

impl Path {

    /// Return the current path position.
    pub fn point(&self) -> Option<IVec3> {
        self.points.get(self.index).copied()
    }

    /// Advanced the path by one point.
    pub fn advance(&mut self) {
        self.index += 1;
    }
    
}
