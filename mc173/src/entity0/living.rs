use std::f32;

use glam::IVec3;

use crate::entity0::base::HurtReason;
use crate::world::World;

use super::BaseClass;


/// The data common to all living entities.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct LivingClass<S> {
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
    /// Apply damage to this entity, after buffering by the living entity implementation.
    pub f_damage: fn(&mut BaseClass<Self>, &mut World, u32, damage: u16),
    /// The sub entity.
    pub sub: S,
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

impl<S> BaseClass<LivingClass<S>> {

    /// The initial hurt time applied on the first damage.
    const HURT_TIME_INIT: u16 = 20;
    const HURT_TIME_LIMIT: u16 = 10;

    /// Create a new living-based entity.
    pub fn living_new(sub: S) -> Self {
        let mut ret = Self::base_new(LivingClass {
            artificial: false,
            health: 10,
            wander_time: 0,
            hurt_time: 0,
            hurt_damage: 0,
            hurt_yaw: 0.0,
            attack_time: 0,
            death_time: 0,
            f_damage: Self::living_damage,
            sub,
        });
        ret.f_hurt = Self::living_hurt;
        ret
    }

    pub fn living_tick(&mut self, world: &mut World, id: u32) {

        // EntityLiving::onUpdate
        // +- Entity::onUpdate
        // |  +- EntityLiving::onEntityUpdate
        // |     +- Entity::onEntityUpdate      (base_tick)
        // |     +- ...
        // +- EntityLiving::onLivingUpdate
        // +- ...

        self.base_tick(world, id);

    }

    pub fn living_hurt(&mut self, world: &mut World, id: u32, damage: u16, reason: HurtReason) -> bool {

        self.sub.wander_time = 0;
        if self.sub.health == 0 {
            return false;
        }

        let first_damage;
        if self.sub.hurt_time > Self::HURT_TIME_LIMIT {
            if damage <= self.sub.hurt_damage {
                return false;
            }
            let delta = damage - self.sub.hurt_damage;
            (self.sub.f_damage)(self, world, id, delta);
            first_damage = false;
        } else {
            self.sub.hurt_damage = damage;
            self.sub.hurt_time = Self::HURT_TIME_INIT;
            (self.sub.f_damage)(self, world, id, damage);
            first_damage = true;
        }

        self.sub.hurt_yaw = 0.0;
        if first_damage {

            if let HurtReason::Entity(reason_id) = reason
            && let Some(entity) = world.get_entity(reason_id) {
                
            } else {
                // PARITY: The Notchian impl is calculating degrees and rounding up things
                // so it has less 
                self.sub.hurt_yaw = world.get_rand_mut().next_double() as f32 * f32::consts::TAU;
            }

        }

        todo!()

    }

    pub fn living_damage(&mut self, _world: &mut World, _id: u32, damage: u16) {
        self.sub.health = self.sub.health.saturating_sub(damage);
    }

}
