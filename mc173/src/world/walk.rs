//! Walking on blocks, support for entities.

use glam::IVec3;

use crate::entity0::base::Base;
use crate::block;

use super::World;


/// Methods related to block walking by entities.
impl World {

    /// Make the given entity walk on the given block.
    /// It returns true if the block below was not air.
    pub fn walk_block(&mut self, pos: IVec3, entity: &mut Base) -> bool {
        if let Some((block::FENCE, _)) = self.get_block(pos - IVec3::Y) {
            // Special handling, this is implemented into Entity::moveEntity, if the block 
            // below the given pos is a fence, then we just do not trigger the block.
            // PARITY: In the real implementation, we just trigger a notification for
            // fence, but on the original block's position, this is good because
            // fence don't have an implementation for this. So we simplify and just do
            // nothing!
            true
        } else if let Some((id, metadata)) = self.get_block(pos) && id != block::AIR {
            self.walk_block_unchecked(pos, id, metadata, entity);
            true
        } else {
            false
        }
    }

    /// This function is unchecked because the caller should ensure that the given id
    /// and metadata is coherent with the given position.
    pub fn walk_block_unchecked(&mut self, pos: IVec3, id: u8, _metadata: u8, entity: &mut Base) {
        match id {
            block::FARMLAND => {
                if entity.rand.next_int_bounded(4) == 0 {
                    self.set_block_notify(pos, block::DIRT, 0);
                }
            }
            block::REDSTONE_ORE => {
                self.set_block_notify(pos, block::REDSTONE_ORE_LIT, 0);
            }
            _ => {}
        }
    }

}
