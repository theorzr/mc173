//! Implementation of the item entity.

use glam::{DVec3, IVec3, Vec3};

use crate::block::material::Material;
use crate::entity0::base::BaseTickOptions;
use crate::geom::Face;
use crate::item::ItemStack;
use crate::world::World;
use crate::block;

use super::base::Base;


/// A full item entity.
#[derive(Debug)]
pub struct Painting {
    /// The base.
    pub base: Base,
    /// Block position of this painting.
    pub pos: IVec3,
    /// The face of the block position the painting is on. Should not be on Y axis.
    pub face: Face,
    /// The art of the painting, which define its size.
    pub art: PaintingArt,
}

impl Painting {

    const CHECK_VALID_INTERVAL: u8 = 101;

    pub fn new(art: PaintingArt) -> Self {
        Self {
            base: Base {
                ..Default::default()
            },
            pos: IVec3::ZERO,
            face: Face::NegY,
            art,
        }
    }

    /// Set the position of this paintaing.
    pub fn set_pos(&mut self, pos: IVec3, face: Face) {
        self.pos = pos;
        self.face = face;
        self.update_pos();
    }

    pub fn set_art(&mut self, art: PaintingArt) {
        self.art = art;
        self.update_pos();
    }

    fn update_pos(&mut self) {

        // Initial position is within the block the painting is placed on.
        self.base.pos = self.pos.as_dvec3() + 0.5;
        // Move its position on the face of the block (1.0 / 16.0 from face).
        self.base.pos += self.face.delta().as_dvec3() * 0.5625;

        let (width, height) = self.art.size();

        // If width is even, the painting cannot be centered on a block, so we move it
        // to center it between two blocks.
        if width % 2 == 0 {
            self.base.pos += self.face.rotate_left().delta().as_dvec3() * 0.5;
        }

        // If height is even, same as above.
        if height % 2 == 0 {
            self.base.pos.y += 0.5;
        }

        let mut size = DVec3::new(width as f64, height as f64, width as f64);
        size[self.face.axis_index()] = 0.03125;
        size -= 0.0125;
        
        self.base.bb.min = self.base.pos - size / 2.0;
        self.base.bb.max = self.base.pos + size / 2.0;

    }

    /// Check the placement of this painting in the world.
    pub fn check_placement(&self, world: &World) -> PaintingPlacement {

        // FIXME: Check that this effectively collides with hard boxes + paintings.
        if world.iter_hard_boxes_colliding(self.base.bb).next().is_some() {
            return PaintingPlacement::Colliding;
        }

        let min = self.base.bb.min.floor().as_ivec3() - self.face.delta();
        let max = self.base.bb.max.floor().as_ivec3() - self.face.delta() + IVec3::ONE;
        for (_, id, _) in world.iter_blocks_in(min, max) {
            if !block::material::get_material(id).is_solid() {
                return PaintingPlacement::Hanging;
            }
        }

        PaintingPlacement::Valid

    }

    /// This this item entity.
    pub fn tick(&mut self, world: &mut World, id: u32) {

        self.base.lifetime += 1;
        if self.base.lifetime % Self::CHECK_VALID_INTERVAL as u32 == 0 {
            self.check_placement(world)
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
