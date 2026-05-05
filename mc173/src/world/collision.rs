//! Collision with block, support for entities.

use glam::IVec3;

use crate::entity1::{Entity, HurtReason};
use crate::block;

use super::World;


/// Methods related to block collision by entities.
impl World {

    /// Make the given entity walk collide with a block.
    pub fn collide_block(&mut self, pos: IVec3, entity: &mut Entity, entity_id: u32) {
        if let Some((id, metadata)) = self.get_block(pos) {
            self.collide_block_unchecked(pos, id, metadata, entity, entity_id);
        }
    }

    /// This function is unchecked because the caller should ensure that the given id
    /// and metadata is coherent with the given position.
    pub fn collide_block_unchecked(&mut self, pos: IVec3, id: u8, _metadata: u8, entity: &mut Entity, entity_id: u32) {
        match id {
            block::CACTUS => {
                entity.hurt(self, entity_id, 1, HurtReason::Cactus);
            }
            block::DETECTOR_RAIL => {
                // TODO:
            }
            block::PORTAL => {
                // TODO:
            }
            block::WOOD_PRESSURE_PLATE => {
                // TODO:
            }
            block::STONE_PRESSURE_PLATE => {
                // TODO:
            }
            block::SOULSAND => {
                entity.vel *= 0.4;
            }
            block::COBWEB => {
                entity.in_cobweb = true;
            }
            _ => {}
        }
    }

}
