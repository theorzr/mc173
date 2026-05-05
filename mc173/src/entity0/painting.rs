use glam::{DVec3, IVec3};

use crate::geom::Face;
use crate::item::{self, ItemStack};
use crate::world::World;
use crate::block;

use super::{EntityDef, BaseClass, Item, HurtReason};


/// A concrete painting entity.
pub type Painting = BaseClass<PaintingClass>;

/// The painting entity class.
#[derive(Debug, Clone)]
pub struct PaintingClass {
    /// Block position of this painting.
    pub block_pos: IVec3,
    /// The face of the block position the painting is on. Should not be on Y axis.
    pub face: Face,
    /// The art of the painting, which define its size.
    pub art: PaintingArt,
}

impl BaseClass<PaintingClass> {

    /// The value is weird, but the Notchian impl is using field++ == 100, so this
    /// actually have an interval of 101, and we are using the lifetime, instead of a
    /// specific field.
    const CHECK_VALID_INTERVAL: u8 = 101;

    /// Create a new painting at 0/0/0 and uninitialized.
    pub fn new(art: PaintingArt) -> Self {
        let mut ret = Self::base_new(PaintingClass {
            block_pos: IVec3::ZERO,
            face: Face::NegY,
            art,
        });
        ret.f_hurt = Self::hurt;
        ret
    }

    /// Set the position of this painting.
    pub fn set_block_pos(&mut self, pos: IVec3, face: Face) {
        self.sub.block_pos = pos;
        self.sub.face = face;
        self.update_pos();
    }

    /// Set the art of this painting.
    pub fn set_art(&mut self, art: PaintingArt) {
        self.sub.art = art;
        self.update_pos();
    }

    fn update_pos(&mut self) {

        // Initial position is within the block the painting is placed on.
        self.pos = self.sub.block_pos.as_dvec3() + 0.5;
        // Move its position on the face of the block (1.0 / 16.0 from face).
        self.pos += self.sub.face.delta().as_dvec3() * 0.5625;

        let (width, height) = self.sub.art.size();

        // If width is even, the painting cannot be centered on a block, so we move it
        // to center it between two blocks.
        if width % 2 == 0 {
            self.pos += self.sub.face.rotate_left().delta().as_dvec3() * 0.5;
        }

        // If height is even, same as above.
        if height % 2 == 0 {
            self.pos.y += 0.5;
        }

        let mut size = DVec3::new(width as f64, height as f64, width as f64);
        size[self.sub.face.axis_index()] = 0.03125;
        size -= 0.0125;
        
        self.bb.min = self.pos - size / 2.0;
        self.bb.max = self.pos + size / 2.0;

    }

    /// Check the placement of this painting in the world.
    pub fn check_placement(&self, world: &World) -> PaintingPlacement {

        // FIXME: Check that this effectively collides with hard boxes + paintings.
        if world.iter_hard_boxes_colliding(self.bb).next().is_some() {
            return PaintingPlacement::Colliding;
        }

        let min = self.bb.min.floor().as_ivec3() - self.sub.face.delta();
        let max = self.bb.max.floor().as_ivec3() - self.sub.face.delta() + IVec3::ONE;
        for (_, id, _) in world.iter_blocks_in(min, max) {
            if !block::material::get_material(id).is_solid() {
                return PaintingPlacement::Hanging;
            }
        }

        PaintingPlacement::Valid

    }

    /// Remove this painting from the world and drop an painting item.
    fn remove_and_drop(&mut self, world: &mut World, id: u32, reason: &str, check_already_dead: bool) {
        if world.remove_entity(id, reason) || !check_already_dead {
            let mut item = Item::new(ItemStack::new(item::PAINTING, 0));
            item.set_pos(self.pos);
            // FIXME:
            // world.spawn_entity(Entity::Item(item));
        }
    }

}

impl EntityDef for BaseClass<PaintingClass> {

    fn set_pos(&mut self, pos: DVec3) {
        self.set_block_pos(pos.floor().as_ivec3(), Face::NegZ);
    }

    fn tick(&mut self, world: &mut World, id: u32) {
        self.lifetime += 1;
        if self.lifetime % Self::CHECK_VALID_INTERVAL as u32 == 0 {
            if self.check_placement(world) != PaintingPlacement::Valid {
                self.remove_and_drop(world, id, "hanging painting", false);
            }
        }
    }

    fn move_by(&mut self, world: &mut World, id: u32, delta: DVec3) {
        if delta.length_squared() > 0.0 {
            self.remove_and_drop(world, id, "moved painting", false);
        }
    }

    fn accel_by(&mut self, world: &mut World, id: u32, vel: DVec3) {
        if vel.length_squared() > 0.0 {
            self.remove_and_drop(world, id, "accel painting", false);
        }
    }

    fn hurt(&mut self, world: &mut World, id: u32, _damage: u16, _reason: HurtReason) -> bool {
        self.remove_and_drop(world, id, "hurt painting", true);
        true
    }

}

