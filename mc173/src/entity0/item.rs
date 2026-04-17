//! Implementation of the item entity.

use glam::{DVec3, IVec3};

use crate::entity0::base::{BaseDef, HurtReason};
use crate::block::material::Material;
use crate::item::ItemStack;
use crate::world::World;
use crate::block;

use super::base::Base;


/// A full item entity.
#[derive(Debug)]
#[repr(C)]
pub struct Item {
    pub base: Base,
    pub inner: Inner,
}

#[derive(Debug)]
struct Inner {
    /// The item stack represented by this entity.
    pub stack: ItemStack,
    /// The item health.
    pub health: u16,
    /// Remaining time for this item to be picked up by entities that have `can_pickup`.
    pub frozen_time: u32,
}

impl Item {

    const SIZE: f32 = 0.25;
    const MAX_LIFETIME: u32 = 6000;

    pub fn new(stack: ItemStack) -> Self {
        Self {
            base: Base {
                ..Default::default()
            },
            inner: Inner {
                stack,
                health: 5,
                frozen_time: 0,
            },
        }
    }

    /// Set position of the item, notes that the item bounding box is 0.25 block wide and
    /// centered around the position.
    pub fn set_pos(&mut self, pos: DVec3) {
        self.base.set_pos(pos, Self::SIZE, Self::SIZE, Self::SIZE / 2.0);
    }

}

impl BaseDef for Item {

    #[inline]
    fn base(&self) -> &Base {
        &self.base
    }

    #[inline]
    fn base_mut(&mut self) -> &mut Base {
        &mut self.base
    }

    fn tick(&mut self, world: &mut World, id: u32) {

        self.tick_(world, id);

        self.inner.frozen_time = self.inner.frozen_time.saturating_sub(1);
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

        self.base.move_out_of_block(world);
        self.base.move_by(world, self.base.vel, Self::SIZE / 2.0, 0.0, false);

        let mut vel_xz_factor = 0.98;
        if self.base.on_ground {
            vel_xz_factor = 0.1 * 0.1 * 58.8;
            let below_pos = IVec3 {
                x: self.base.pos.x.floor() as i32,
                y: self.base.bb.min.y.floor() as i32 - 1,
                z: self.base.pos.z.floor() as i32,
            };
            if let Some((below_id, _)) = world.get_block(below_pos)
            && below_id != block::AIR {
                vel_xz_factor = block::material::get_slipperiness(below_id) * 0.98;
            }
        }

        self.base.vel *= DVec3::new(vel_xz_factor as f64, 0.98, vel_xz_factor as f64);
        if self.base.on_ground {
            self.base.vel.y *= -0.5;
        }

        if self.base.lifetime > Self::MAX_LIFETIME {
            world.remove_entity(id, "item lifetime");
        }

    }

    fn hurt(&mut self, world: &mut World, id: u32, damage: u16, _reason: HurtReason) -> bool {
        self.inner.health = self.inner.health.saturating_sub(damage);
        if self.inner.health <= 0 {
            world.remove_entity(id, "health");
        }
        false
    }

}
