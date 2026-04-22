//! New module for the entity implementation, to simplify it!

use enum_dispatch::enum_dispatch;
use glam::DVec3;

use crate::world::World;

mod base;
mod item;
mod painting;
mod boat;
mod living;

pub use base::{BaseClass, HurtReason};
pub use item::{Item, ItemClass};
pub use painting::{Painting, PaintingClass, PaintingArt, PaintingPlacement};
pub use boat::Boat;


/// This trait describes the required methods for an entity to implements.;dfw
#[enum_dispatch]
pub trait EntityDef {
    fn set_pos(&mut self, pos: DVec3);
    fn tick(&mut self, world: &mut World, id: u32);
    fn move_by(&mut self, world: &mut World, id: u32, delta: DVec3);
    fn accel_by(&mut self, world: &mut World, id: u32, vel: DVec3);
    fn hurt(&mut self, world: &mut World, id: u32, damage: u16, reason: HurtReason) -> bool;
}

#[enum_dispatch(EntityDef)]
#[derive(Debug, Clone)]
pub enum Entity {
    Item,
    Painting,
    Boat,
    // Minecart,
    // Bobber,
    // LightningBolt,
    // FallingBlock,
    // Tnt,
    // Arrow,
    // Egg,
    // Fireball,
    // Snowball,
    // Human,
    // Ghast,
    // Slime,
    // Pig,
    // Chicken,
    // Cow,
    // Sheep,
    // Squid,
    // Wolf,
    // Creeper,
    // Giant,
    // PigZombie,
    // Skeleton,
    // Spider,
    // Zombie,
}
