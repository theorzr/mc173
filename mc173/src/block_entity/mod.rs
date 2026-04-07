//! Block entities structures and logic implementation.

use glam::IVec3;

use crate::world::World;

pub mod chest;
pub mod furnace;
pub mod dispenser;
pub mod spawner;
pub mod note_block;
pub mod piston;
pub mod sign;
pub mod jukebox;


/// All kinds of block entities.
#[derive(Debug, Clone)]
pub enum BlockEntity {
    Chest(chest::Chest),
    Furnace(furnace::Furnace),
    Dispenser(dispenser::Dispenser),
    Spawner(spawner::Spawner),
    NoteBlock(note_block::NoteBlock),
    Piston(piston::Piston),
    Sign(sign::Sign),
    Jukebox(jukebox::Jukebox),
}

impl BlockEntity {

    /// Tick the block entity at its position in the world.
    pub fn tick(&mut self, world: &mut World, pos: IVec3) {
        match self {
            BlockEntity::Chest(_) => (),
            BlockEntity::Furnace(furnace) => furnace.tick(world, pos),
            BlockEntity::Dispenser(_) => (),
            BlockEntity::Spawner(spawner) => spawner.tick(world, pos),
            BlockEntity::NoteBlock(_) => (),
            BlockEntity::Piston(piston) => piston.tick(world, pos),
            BlockEntity::Sign(_) => (),
            BlockEntity::Jukebox(_) => (),
        }
    }

}
