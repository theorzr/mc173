use glam::{DVec3, IVec3};

use crate::item::{self, ItemStack};
use crate::world::World;
use crate::geom::Face;
use crate::block;

use super::{Entity, EntityKind};


crate::class::class! {
    pub struct Painting: Entity {
        /// Block position of this painting.
        pub block_pos: IVec3,
        /// The face of the block position the painting is on. Should not be on Y axis.
        pub face: Face = Face::NegZ,
        /// The art of the painting, which define its size.
        pub art: PaintingArt,
    }
}

impl Painting {

    /// The value is weird, but the Notchian impl is using field++ == 100, so this
    /// actually have an interval of 101, and we are using the lifetime, instead of a
    /// specific field.
    const CHECK_VALID_INTERVAL: u8 = 101;

    /// Set the position and face of this painting.
    pub fn set_pos_and_face(&mut self, pos: IVec3, face: Face) {
        self.block_pos = pos;
        self.face = face;
        self.update_pos();
    }

    /// Set the position of this painting.
    pub fn set_pos(&mut self, pos: IVec3) {
        self.block_pos = pos;
        self.update_pos();
    }

    /// Set the face of this painting.
    pub fn set_face(&mut self, face: Face) {
        self.face = face;
        self.update_pos();
    }

    /// Set the art of this painting.
    pub fn set_art(&mut self, art: PaintingArt) {
        self.art = art;
        self.update_pos();
    }

    pub fn tick(&mut self, world: &mut World, id: u32) {
        self.lifetime += 1;
        if self.lifetime % Self::CHECK_VALID_INTERVAL as u32 == 0 {
            if self.check_placement(world) != PaintingPlacement::Valid {
                self.remove_and_drop(world, id, "hanging painting", false);
            }
        }
    }

    pub fn move_by(&mut self, world: &mut World, id: u32, delta: DVec3) {
        if delta.length_squared() > 0.0 {
            self.remove_and_drop(world, id, "moved painting", false);
        }
    }

    pub fn accel_by(&mut self, world: &mut World, id: u32, vel: DVec3) {
        if vel.length_squared() > 0.0 {
            self.remove_and_drop(world, id, "accel painting", false);
        }
    }

    /// Check the placement of this painting in the world.
    pub fn check_placement(&self, world: &World) -> PaintingPlacement {

        if world.iter_hard_boxes_colliding(self.bb, false).next().is_some() {
            return PaintingPlacement::Colliding;
        }

        // Maybe this could be simplified and integrated into 'iter_hard_boxes_colliding'
        for (_, entity) in world.iter_entities_colliding(self.bb) {
            if entity.kind() == EntityKind::Painting {
                return PaintingPlacement::Colliding;
            }
        }

        let min = self.bb.min.floor().as_ivec3() - self.face.delta();
        let max = self.bb.max.floor().as_ivec3() - self.face.delta() + IVec3::ONE;
        for (_, id, _) in world.iter_blocks_in(min, max) {
            if !block::material::get_material(id).is_solid() {
                return PaintingPlacement::Hanging;
            }
        }

        PaintingPlacement::Valid

    }

    fn update_pos(&mut self) {

        let face = self.face;

        // Initial position is within the block the painting is placed on.
        self.pos = self.block_pos.as_dvec3() + 0.5;
        // Move its position on the face of the block (1.0 / 16.0 from face).
        self.pos += face.delta().as_dvec3() * 0.5625;

        let (width, height) = self.art.size();

        // If width is even, the painting cannot be centered on a block, so we move it
        // to center it between two blocks.
        if width % 2 == 0 {
            self.pos += face.rotate_left().delta().as_dvec3() * 0.5;
        }

        // If height is even, same as above.
        if height % 2 == 0 {
            self.pos.y += 0.5;
        }

        let mut size = DVec3::new(width as f64, height as f64, width as f64);
        size[face.axis_index()] = 0.03125;
        size -= 0.0125;
        
        self.bb.min = self.pos - size / 2.0;
        self.bb.max = self.pos + size / 2.0;

    }

    /// Remove this painting from the world and drop an painting item.
    fn remove_and_drop(&mut self, world: &mut World, id: u32, reason: &str, check_already_dead: bool) {
        if world.remove_entity(id, reason) || !check_already_dead {
            world.spawn_loot(self.pos, ItemStack::new(item::PAINTING, 0), 0.0, 0);
        }
    }

}

/// Represent the art type for a painting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PaintingArt {
    #[default]
    Kebab,
    Aztec,
    Alban,
    Aztec2,
    Bomb,
    Plant,
    Wasteland,
    Pool,
    Courbet,
    Sea,
    Sunset,
    Creebet,
    Wanderer,
    Graham,
    Match,
    Bust,
    Stage,
    Void,
    SkullAndRoses,
    Fighters,
    Pointer,
    Pigscene,
    BurningSkull,
    Skeleton,
    DonkeyKong,
}

impl PaintingArt {

    pub const ALL: [PaintingArt; 25] = [
        Self::Kebab,
        Self::Aztec,
        Self::Alban,
        Self::Aztec2,
        Self::Bomb,
        Self::Plant,
        Self::Wasteland,
        Self::Pool,
        Self::Courbet,
        Self::Sea,
        Self::Sunset,
        Self::Creebet,
        Self::Wanderer,
        Self::Graham,
        Self::Match,
        Self::Bust,
        Self::Stage,
        Self::Void,
        Self::SkullAndRoses,
        Self::Fighters,
        Self::Pointer,
        Self::Pigscene,
        Self::BurningSkull,
        Self::Skeleton,
        Self::DonkeyKong,
    ];

    /// Return the size of the painting, in blocks (width, height).
    pub const fn size(self) -> (u8, u8) {
        match self {
            Self::Kebab => (1, 1),
            Self::Aztec => (1, 1),
            Self::Alban => (1, 1),
            Self::Aztec2 => (1, 1),
            Self::Bomb => (1, 1),
            Self::Plant => (1, 1),
            Self::Wasteland => (1, 1),
            Self::Pool => (2, 1),
            Self::Courbet => (2, 1),
            Self::Sea => (2, 1),
            Self::Sunset => (2, 1),
            Self::Creebet => (2, 1),
            Self::Wanderer => (1, 2),
            Self::Graham => (1, 2),
            Self::Match => (2, 2),
            Self::Bust => (2, 2),
            Self::Stage => (2, 2),
            Self::Void => (2, 2),
            Self::SkullAndRoses => (2, 2),
            Self::Fighters => (4, 2),
            Self::Pointer => (4, 4),
            Self::Pigscene => (4, 4),
            Self::BurningSkull => (4, 4),
            Self::Skeleton => (4, 3),
            Self::DonkeyKong => (4, 3),
        }
    }

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintingPlacement {
    Valid,
    Colliding,
    Hanging,
}
