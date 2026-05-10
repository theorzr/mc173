use glam::{DVec3, IVec3};

use crate::block::material::Material;
use crate::item::ItemStack;
use crate::world::World;
use crate::block;

use super::{Entity, HurtReason};


const SIZE: f32 = 0.25;
const MAX_LIFETIME: u32 = 6000;


crate::class::class! {
    pub struct Item: Entity {
        /// The item stack represented by this entity.
        pub stack: ItemStack,
        /// The item health.
        pub health: u16 = 5,
        /// Remaining time for this item to be picked up by entities that have `can_pickup`.
        pub frozen_time: u32,
    }
}

impl Item {

    pub fn tick(&mut self, world: &mut World, id: u32) {

        self.super_mut()._tick(world, id);

        self.frozen_time = self.frozen_time.saturating_sub(1);
        self.vel.y -= 0.04;

        // Handle item in lava, note that the notchian implementation does not use the
        // 'in_lava' state, and it could be slightly different since it checks if any of
        // the bounding box is colliding.
        if world.get_block_material(self.pos.floor().as_ivec3()) == Material::Lava {
            self.vel = DVec3 {
                x: ((self.rand.next_float() - self.rand.next_float()) * 0.2) as f64,
                y: 0.2,
                z: ((self.rand.next_float() - self.rand.next_float()) * 0.2) as f64,
            };
            // A dummy next float here because it's used, even in the server code, to 
            // compute the volume or pitch of the sound.
            let _ = self.rand.next_float();
        }

        self.super_mut().move_out_of_block(world);

        let vel = self.vel;
        self.super_mut().move_by(world, id, vel);

        let mut vel_xz_factor = 0.98;
        if self.on_ground {
            vel_xz_factor = 0.1 * 0.1 * 58.8;
            let below_pos = IVec3 {
                x: self.pos.x.floor() as i32,
                y: self.bb.min.y.floor() as i32 - 1,
                z: self.pos.z.floor() as i32,
            };
            if let Some((below_id, _)) = world.get_block(below_pos)
            && below_id != block::AIR {
                vel_xz_factor = block::material::get_slipperiness(below_id) * 0.98;
            }
        }

        self.vel *= DVec3::new(vel_xz_factor as f64, 0.98, vel_xz_factor as f64);
        if self.on_ground {
            self.vel.y *= -0.5;
        }

        if self.lifetime > MAX_LIFETIME {
            world.remove_entity(id, "item lifetime");
        }

    }

    pub fn handle_water_vel(&mut self, world: &mut World, _id: u32) -> Option<DVec3> {
        super::calc_fluid_vel_in_box(world, self.bb, Material::Water)
    }

    pub fn hurt(&mut self, world: &mut World, id: u32, damage: u16, _reason: HurtReason) -> bool {

        self.health = self.health.saturating_sub(damage);
        if self.health <= 0 {
            world.remove_entity(id, "item dead");
        }

        false

    }

}
