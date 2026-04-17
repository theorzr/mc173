//! New module for the entity implementation, to simplify it!

use glam::{DVec3, Vec2};

use crate::geom::BoundingBox;
use crate::java::JavaRandom;
use crate::world::World;

pub mod base;
pub mod item;
pub mod painting;

pub mod living;


/// An entity.
#[derive(Debug)]
pub enum Entity {
    Item(item::Item),
    Painting,
    Boat,
    Minecart,
    Bobber,
    LightningBolt,
    FallingBlock,
    Tnt,
    Arrow,
    Egg,
    Fireball,
    Snowball,
    Human,
    Ghast,
    Slime,
    Pig,
    Chicken,
    Cow,
    Sheep,
    Squid,
    Wolf,
    Creeper,
    Giant,
    PigZombie,
    Skeleton,
    Spider,
    Zombie,
}
