use glam::{DVec3, IVec3};

use crate::block;
use crate::block::material::Material;
use crate::item::ItemStack;
use crate::world::World;

use super::{base, EntityDef, BaseClass, HurtReason};


/// A concret item entity.
pub type Item = BaseClass<ItemClass>;

/// The item class.
#[derive(Debug, Clone)]
pub struct ItemClass {
    /// The item stack represented by this entity.
    pub stack: ItemStack,
    /// The item health.
    pub health: u16,
    /// Remaining time for this item to be picked up by entities that have `can_pickup`.
    pub frozen_time: u32,
}

impl BaseClass<ItemClass> {

    const SIZE: f32 = 0.25;
    const MAX_LIFETIME: u32 = 6000;
    
    pub fn new(stack: ItemStack) -> Self {
        let mut ret = Self::base_new(ItemClass {
            stack,
            health: 5,
            frozen_time: 0,
        });
        ret.f_handle_water_vel = Self::item_handle_water_vel;
        ret.f_hurt = Self::hurt;
        ret
    }

    fn item_handle_water_vel(&mut self, world: &mut World, _id: u32) -> Option<DVec3> {
        base::calc_fluid_vel_in_box(world, self.bb, Material::Water)
    }

}

impl EntityDef for BaseClass<ItemClass> {

    fn set_pos(&mut self, pos: DVec3) {
        self.base_set_pos(pos, Self::SIZE, Self::SIZE, Self::SIZE / 2.0);
    }

    fn tick(&mut self, world: &mut World, id: u32) {

        self.base_tick(world, id);

        self.sub.frozen_time = self.sub.frozen_time.saturating_sub(1);
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

        self.base_move_out_of_block(world);
        self.move_by(world, id, self.vel);

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

        if self.lifetime > Self::MAX_LIFETIME {
            world.remove_entity(id, "item lifetime");
        }

    }

    fn move_by(&mut self, world: &mut World, id:u32, delta: DVec3) {
        self.base_move_by(world, id, delta, Self::SIZE / 2.0, 0.0, false);
    }

    fn accel_by(&mut self, world: &mut World, id: u32, vel: DVec3) {
        self.base_accel_by(world, id, vel);
    }

    fn hurt(&mut self, world: &mut World, id: u32, damage: u16, _reason: HurtReason) -> bool {
        self.sub.health = self.sub.health.saturating_sub(damage);
        if self.sub.health <= 0 {
            world.remove_entity(id, "dead");
        }
        false
    }

}
