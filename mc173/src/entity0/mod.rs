//! New module for the entity implementation, to simplify it!
//! 
//! Here is a hierarchy I made to understand the overriding of methods in the entity
//! class hierarchy. This is very important to ensure that we implement the same order
//! for entity ticking.
//! 
//! Entity::onUpdate
//!     Entity::onEntityUpdate
//!         Entity::handleWaterMovement
//!         Entity::attackEntityFrom  # fire damage
//!         Entity::handleLavaMovement
//!         Entity::setOnFireFromLava  # see above
//!             Entity::attackEntityFrom
//!         Entity::kill  # y < -64
//!             Entity::setEntityDead
//! 
//! Item::onUpdate
//!     Entity::onUpdate
//!     Entity::moveEntityOutOfBlock
//!     Entity::moveEntity
//! 
//! Painting::onUpdate
//!     Painting::onValidSurface
//!     Entity::setEntityDead  # not a valid surface
//! Painting::attackEntityFrom
//!     Entity::setEntityDead
//! Painting::moveEntity
//!     Entity::setEntityDead
//! Painting::addVelocity
//!     Entity::setEntityDead
//! 
//! 
//! 
//! 

use glam::{DVec3, IVec3};

pub mod base;
pub mod item;

pub mod living;


/// An enumeration of all entity types.
#[derive(Debug, Clone)]
pub enum Entity {
    Item,
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

impl Entity {

    pub fn set_pos(&mut self, pos: IVec3) {
        todo!()
    }

    pub fn change_vel(&mut self, factor: DVec3) {
        todo!()
    }

    pub fn set_in_cobweb(&mut self) {
        todo!()
    }

    pub fn hurt(&mut self, damage: u16, origin: Option<u32>) {
        todo!()
    }

}
