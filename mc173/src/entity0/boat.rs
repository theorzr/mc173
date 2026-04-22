use glam::DVec3;

use crate::{block, item};
use crate::block::material::Material;
use crate::item::ItemStack;
use crate::world::World;

use super::{base, EntityDef, BaseClass, HurtReason, Item};


/// A concret boat entity.
pub type Boat = BaseClass<BoatClass>;

/// The boat class.
#[derive(Debug, Clone)]
pub struct BoatClass {
    damage: u16,
}

impl BaseClass<BoatClass> {

    const WIDTH: f32 = 1.5;
    const HEIGHT: f32 = 0.6;

    pub fn new() -> Self {
        let mut ret = Self::base_new(BoatClass {
            damage: 0,
        });
        ret.f_hurt = Self::hurt;
        ret
    }

    fn remove_and_drop(&mut self, world: &mut World, id: u32, reason: &str) {

        world.remove_entity(id, reason);

        for _ in 0..3 {
            let mut item = Item::new(ItemStack::new_block(block::WOOD, 0));
            item.set_pos(self.pos);
            // TODO: spawn
        }

        for _ in 0..2 {
            let mut item = Item::new(ItemStack::new(item::STICK, 0));
            item.set_pos(self.pos);
            // TODO: spawn
        }

    }

}

impl EntityDef for BaseClass<BoatClass> {

    fn set_pos(&mut self, pos: DVec3) {
        self.base_set_pos(pos, Self::WIDTH, Self::HEIGHT, Self::HEIGHT / 2.0);
    }

    fn tick(&mut self, world: &mut World, id: u32) {

        self.base_tick(world, id);

        self.sub.damage = self.sub.damage.saturating_sub(1);

        let in_water_divisions = 5u8;
        let mut in_water_factor = 0.0;
        for i in 0..in_water_divisions {
            let mut bb = self.bb;
            bb.min.y = bb.min.y + bb.size_y() * (i + 0) as f64 / in_water_divisions as f64 - 0.125;
            bb.max.y = bb.min.y + bb.size_y() * (i + 1) as f64 / in_water_divisions as f64 - 0.125;
            if base::is_fluid_in_box(world, bb, Material::Water) {
                in_water_factor += 1.0 / in_water_divisions as f64;
            }
        }

        if in_water_factor < 1.0 {
            self.vel.y += 0.04 * (in_water_factor * 2.0 - 1.0);
        } else {
            if self.vel.y < 0.0 {
                self.vel.y /= 2.0;
            }
            self.vel.y += 0.007;
        }

        // TODO: Change motion depending on the motion of ridding entity.
        if let Some(rider_id) = self.rider_id
        && let Some(_rider_entity) = world.get_entity(rider_id) {
            // TODO:
        }

        self.vel.x = self.vel.x.clamp(-0.4, 0.4);
        self.vel.y = self.vel.y.clamp(-0.4, 0.4);

        if self.on_ground {
            self.vel *= 0.5;
        }

        let prev_pos = self.pos;
        self.move_by(world, id, self.vel);

        // Here we just simulate the calls to rand, but it's only used for particles, so
        // this is only effective to the client side.
        let horizontal_speed = f64::sqrt(self.vel.x * self.vel.x + self.vel.z * self.vel.z);
        if horizontal_speed > 0.15 {
            for _ in 0..(1.0 + horizontal_speed * 60.0).ceil() as u32 {
                let _ = self.rand.next_float();
                let _ = self.rand.next_int_bounded(2);
                let _ = self.rand.next_bool();
            }
        }

        if self.collided_xz && horizontal_speed > 0.15 {
            self.remove_and_drop(world, id, "collide boat");
        } else {
            self.vel *= DVec3::new(0.99, 0.95, 0.99);
        }

        self.pitch = 0.0;

        let delta_x = prev_pos.x - self.pos.x;
        let delta_z = prev_pos.z - self.pos.z;
        if delta_x * delta_x + delta_z * delta_z > 0.001 {
            self.yaw += f64::atan2(delta_x, delta_z).clamp(f64::to_radians(-20.0), f64::to_radians(20.0)) as f32;
        }

        // TODO: Apply entity collision

        // TODO: Break snow layer??

    }

    fn move_by(&mut self, world: &mut World, id: u32, delta: DVec3) {
        self.base_move_by(world, id, delta, Self::HEIGHT / 2.0, 0.0, false);
    }

    fn accel_by(&mut self, world: &mut World, id: u32, vel: DVec3) {
        self.base_accel_by(world, id, vel);
    }

    fn hurt(&mut self, world: &mut World, id: u32, damage: u16, _reason: HurtReason) -> bool {
        // Only if the entity is not already dead...
        if world.contains_entity(id) {
            self.sub.damage += damage * 16;
            if self.sub.damage > 40 {
                // TODO:
                // if(this.riddenByEntity != null) {
				// 	this.riddenByEntity.mountEntity(this);
				// }
                self.remove_and_drop(world, id, "hurt boat");
            }
        }
        true
    }

}
