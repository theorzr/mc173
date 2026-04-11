//! Walking on blocks, support for entities.

use glam::{DVec3, IVec3};

use crate::block;
use crate::entity0::Entity;

use super::World;


/// Methods related to block walking by entities.
impl World {

    /// Make the given entity walk on the given block.
    /// It returns true if the block below was not air.
    pub fn walk_block(&self, pos: IVec3, entity: &mut Entity) -> bool {
        // Special handling, this is implemented into Entity::moveEntity.
        if let Some((block::FENCE, _)) = self.get_block(pos - IVec3::Y) {
            self.walk_block_unchecked(pos, block::FENCE, entity);
            true
        } else {

        }

        let block_below = self.get_block(pos);


        if let Some((id, metadata)) = self.get_block(pos)
        && id != block::AIR {
            self.walk_block_unchecked(pos, id, entity);
            true
        } else {
            false
        }
    }

    pub(super) fn walk_block_unchecked(&mut self, pos: IVec3, id: u8, entity: &mut Entity) {
        match id {
            block::FARMLAND => {
                
            }
        }
    }

}
