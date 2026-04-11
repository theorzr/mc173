//! Implementation of the item entity.

use std::cell::RefCell;

use glam::DVec3;

use crate::block::material::Material;
use crate::entity0::base::BaseTickOptions;
use crate::geom::{BoundingBox, Face};
use crate::item::ItemStack;
use crate::world::World;

use super::base::{self, Base};


/// A full item entity.
#[derive(Debug)]
pub struct Item {
    /// The base.
    pub base: Base,
    /// The item stack represented by this entity.
    pub stack: ItemStack,
    /// The item health.
    pub health: u16,
    /// Remaining time for this item to be picked up by entities that have `can_pickup`.
    pub frozen_time: u32,
}

impl Item {

    pub fn new(stack: ItemStack) -> Self {
        Self {
            base: Base {
                ..Default::default()
            },
            stack,
            health: 5,
            frozen_time: 0,
        }
    }

    /// Set position of the item, notes that the item bounding box is 0.25 block wide and
    /// centered around the position.
    pub fn set_pos(&mut self, pos: DVec3) {
        self.base.set_position(pos, 0.25, 0.25, 0.25 / 2.0);
    }

    /// This this item entity.
    pub fn tick(&mut self, world: &mut World, id: u32) {

        self.base.tick(world, id, &BaseTickOptions {
            water_bb_inflate: DVec3::ZERO,
            damage_in_void: None,
            ..BaseTickOptions::default()
        });

        self.frozen_time = self.frozen_time.saturating_sub(1);
        self.base.vel.y -= 0.04;

        // Handle item in lava, note that the notchian implementation does not use the
        // 'in_lava' state, and it could be slightly different since it checks if any of
        // the bounding box is colliding.
        if world.get_block_material(self.base.pos.floor().as_ivec3()) == Material::Lava {
            self.base.vel = DVec3 {
                x: ((self.base.rand.next_float() - self.base.rand.next_float()) * 0.2) as f64,
                y: 0.2,
                z: ((self.base.rand.next_float() - self.base.rand.next_float()) * 0.2) as f64,
            };
            // A dummy next float here because it's used, even in the server code, to 
            // compute the volume or pitch of the sound.
            let _ = self.base.rand.next_float();
        }

        // If the item is in an opaque block, move it out of the block.
        let block_pos = self.base.pos.floor().as_ivec3();
        if world.is_block_normal_cube(block_pos) {

            let delta = self.base.pos - block_pos.as_dvec3();

            // Find a block face where we can bump the item.
            let bump_face = Face::ALL.into_iter()
                .filter(|face| !world.is_block_normal_cube(block_pos + face.delta()))
                .map(|face| {
                    let mut delta = delta[face.axis_index()];
                    if face.is_pos() {
                        delta = 1.0 - delta;
                    }
                    (face, delta)
                })
                .min_by(|&(_, delta1), &(_, delta2)| delta1.total_cmp(&delta2))
                .map(|(face, _)| face);

            // If we found a non opaque face then we bump the item to that face.
            if let Some(bump_face) = bump_face {
                let accel = (self.base.rand.next_float() * 0.2 + 0.1) as f64;
                if bump_face.is_neg() {
                    self.base.vel[bump_face.axis_index()] = -accel;
                } else {
                    self.base.vel[bump_face.axis_index()] = accel;
                }
            }
            
        }

    }

}
