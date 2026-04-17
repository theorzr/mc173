//! The base entity class.

use std::cell::RefCell;
use std::ops::{Add, Deref, DerefMut, Sub};

use glam::{DVec3, IVec3, Vec2};

use crate::block::material::Material;
use crate::geom::{BoundingBox, Face};
use crate::java::JavaRandom;
use crate::world::{LocalWeather, World};
use crate::block;


/// The base data common to all entities.
#[derive(Debug, Clone)]
#[doc(alias = "notchian/Entity")]
pub struct Base {
    /// Tell if this entity is persistent or not. A persistent entity is saved with its
    /// chunk, but non-persistent entities are no saved. For example, all player entities
    /// are typically non-persistent because these are not real entities. Some entities
    /// cannot be persistent as they are not supported by the Notchian serialization.
    pub persistent: bool,
    /// The bounding box is defining the actual position from the size of the entity, the 
    /// actual position of the entity is derived from it. This is recomputed with the size
    /// by `tick_base` method when entity isn't coherent.
    pub bb: BoundingBox,
    /// The current entity position, it is derived from the bounding box and size, it can
    /// be forced by setting it and then calling `resize` on entity.
    pub pos: DVec3,
    /// When a step is taken by the entity, the bounding box is immediately pushed upward,
    /// but the position of the entity is instead more progressive toward this new pos.
    pub step_progress: f32,
    /// True if an entity pos event should be sent after update.
    /// The current entity velocity.
    pub vel: DVec3,
    /// Yaw and pitch angles of this entity's look. These are in radians with no range 
    /// guarantee, although this will often be normalized in 2pi range. The yaw angle
    /// in Minecraft is set to zero when pointing toward PosZ, and then rotate clockwise
    /// to NegX, NegZ and then PosX.
    /// 
    /// Yaw is X and pitch is Y.
    pub look: Vec2,
    /// Lifetime of the entity since it was spawned in the world, it increase at every
    /// world tick.
    pub lifetime: u32,
    /// No clip is used to disable collision check when moving the entity, if no clip is
    /// false, then the entity will be constrained by bounding box in its way.
    pub no_clip: bool,
    /// True if the entity collided in X and/or Z in the last move.
    pub collided_xz: bool,
    /// True if the entity collided in Y in the last move.
    pub collided_y: bool,
    /// Walk distance, scaled to some factor, when we increment the unit, at least, we'll
    /// interact with the environment. On each tick, if >= 1, 1 is subtracted and a 
    /// walking is triggered on the block below.
    pub walk_dist: f32,
    /// Is this entity currently on ground.
    pub on_ground: bool,
    /// Is this entity moving safely (aka sneaking for players).
    pub sneaking: bool,
    /// Is this entity in water.
    pub in_water: bool,
    /// Is this entity in lava.
    pub in_lava: bool,
    /// Is this entity in cobweb.
    pub in_cobweb: bool,
    /// Total fall distance, will be used upon contact to calculate damages to deal.
    pub fall_distance: f32,
    /// Remaining fire ticks.
    pub fire_time: i32,
    /// The current fire resistance.
    pub fire_resistance: i32,
    /// True if this entity is immune to fire.
    pub fire_immune: bool,
    /// Remaining air ticks to breathe.
    pub air_time: u32,
    /// A list of hurts to apply to the entity.
    pub hurt: Vec<Hurt>,
    /// If this entity is ridden, this contains its entity id.
    pub rider_id: Option<u32>,
    /// If this entity is riding, this contains its entity id.
    pub ridden_id: Option<u32>,
    /// If this entity has thrown a bobber for fishing, this contains its entity id.
    pub bobber_id: Option<u32>,
    /// The random number generator used for this entity.
    pub rand: JavaRandom,
}

/// Hurt data to apply on the next tick to the entity.
#[derive(Debug, Clone)]
pub struct Hurt {
    /// The damage to deal.
    pub damage: u16,
    /// The reason for this hurt. 
    pub reason: HurtReason,
}

/// The different reasons for hurting an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HurtReason {
    Fire,
    Void,
    Entity(i32),
}

impl Default for Base {
    fn default() -> Self {
        Self {
            persistent: false,
            bb: BoundingBox::NULL,
            pos: DVec3::ZERO,
            step_progress: 0.0,
            vel: DVec3::ZERO,
            look: Vec2::ZERO,
            lifetime: 0,
            no_clip: false,
            collided_xz: false,
            collided_y: false,
            walk_dist: 0.0,
            on_ground: false,
            sneaking: false,
            in_water: false,
            in_lava: false,
            in_cobweb: false,
            fall_distance: 0.0,
            fire_time: 0,
            fire_resistance: 1,
            fire_immune: false,
            air_time: 0,
            hurt: Vec::new(),
            rider_id: None,
            ridden_id: None,
            bobber_id: None,
            rand: JavaRandom::new_seeded(),
        }
    }
}

impl Base {

    /// Set the position of this entity, and update the bounding box be centered around
    /// given the given width and height of the entity. By default the bounding box
    /// minimum Y is set to the position Y, the height offset can be used to offset the
    /// minimum Y of the box below that point.
    pub fn set_pos(&mut self, pos: DVec3, width: f32, height: f32, height_offset: f32) {

        self.pos = pos;
        
        let half_width = (width / 2.0) as f64;
        let height = height as f64;
        let height_offset = height_offset as f64;

        self.bb = BoundingBox { 
            min: pos - DVec3::new(half_width, height_offset, half_width), 
            max: pos + DVec3::new(half_width, height - height_offset, half_width),
        };

    }

    pub fn tick<S: DerefMut<Target = Self>>(
        this: &mut S, 
        world: &mut World, 
        id: u32,
        handle_water_vel: fn(this: &mut S, world: &mut World, id: u32),
    ) {
        
    }

    pub fn tick(&mut self, world: &mut World, id: u32) {

        // Handle water
        if let Some(vel) = self.handle_water_vel(world, id) {
            self.vel += vel * 0.014;
            self.in_water = true;
            self.fall_distance = 0.0;
            self.fire_time = 0;
        } else {
            self.in_water = false;
        }
        
        // Handle fire
        if self.fire_time > 0 {
            if self.fire_immune {
                self.fire_time -= 4;
                if self.fire_time < 0 {
                    self.fire_time = 0;
                }
            } else {
                self.fire_time -= 1;
                if (self.fire_time + 1) % 20 == 0 {
                    self.hurt(world, id, 1, HurtReason::Fire);
                }
            }
        }

        // Handle lava
        let lava_bb = self.bb.inflate(DVec3::new(-0.1, -0.4, -0.1));
        self.in_lava = world.iter_blocks_in_box(lava_bb)
            .any(|(_, block, _)| block::material::get_material(block) == Material::Lava);
        if self.in_lava && !self.fire_immune {
            self.fire_time = 600;
        }

        // Handle void
        if self.pos.y < -64.0 {
            self.hurt_void(world, id);
        }

    }

    /// Move the entity by checking its collisions (or ignoring if no clip).
    pub fn move_by(&mut self, 
        world: &mut World, 
        delta: DVec3, 
        height_offset: f32, 
        step_height: f32,
        walk_interact: bool,
    ) {

        // We use a thread local for the bounding box vector.
        thread_local! {
            static COLLIDING_BBS: RefCell<Vec<BoundingBox>> = const { RefCell::new(Vec::new()) };
            static COLLIDING_BLOCKS: RefCell<Vec<(IVec3, u8, u8)>> = const { RefCell::new(Vec::new()) };
        }

        let delta = delta;

        if self.no_clip {
            self.bb += delta;
            self.pos = DVec3 {
                x: self.bb.center_x(),
                y: self.bb.min.y - height_offset as f64,
                z: self.bb.center_y(),
            };
        } else {

            self.step_progress *= 4.0;

            // Handle cobweb...
            if self.in_cobweb {
                self.in_cobweb = false;
                delta = delta * DVec3::new(0.25, 0.05, 0.25);
                self.vel = DVec3::ZERO;
            }

            // Handle sneaking...
            let sneaking_on_ground = self.on_ground && self.sneaking;
            if sneaking_on_ground {
                
                let sneaking_offset = 0.05;
                
                while delta.x != 0.0 && world.iter_hard_boxes_colliding(self.bb + DVec3::new(delta.x, -1.0, 0.0)).next().is_none() {
                    if delta.x < sneaking_offset && delta.x >= -sneaking_offset {
                        delta.x = 0.0;
                    } else if delta.x > 0.0 {
                        delta.x -= sneaking_offset;
                    } else {
                        delta.x += sneaking_offset;
                    }
                }
                
                while delta.z != 0.0 && world.iter_hard_boxes_colliding(self.bb + DVec3::new(0.0, -1.0, delta.z)).next().is_none() {
                    if delta.z < sneaking_offset && delta.z >= -sneaking_offset {
                        delta.z = 0.0;
                    } else if delta.z > 0.0 {
                        delta.z -= sneaking_offset;
                    } else {
                        delta.z += sneaking_offset;
                    }
                }

            }

            // Handle normal collisions...
            let mut new_delta = delta;
            let mut new_bb = self.bb;

            COLLIDING_BBS.with_borrow_mut(|colliding_bbs| {

                debug_assert!(colliding_bbs.is_empty());
                colliding_bbs.extend(world.iter_hard_boxes_colliding(new_bb + new_delta));
                
                // Check collision on Y axis.
                for colliding_bb in &*colliding_bbs {
                    new_delta.y = colliding_bb.calc_y_delta(new_bb, new_delta.y);
                }
                new_bb += DVec3::new(0.0, new_delta.y, 0.0);
        
                // Check collision on X axis.
                for colliding_bb in &*colliding_bbs {
                    new_delta.x = colliding_bb.calc_x_delta(new_bb, new_delta.x);
                }
                new_bb += DVec3::new(new_delta.x, 0.0, 0.0);
        
                // Check collision on Z axis.
                for colliding_bb in &*colliding_bbs {
                    new_delta.z = colliding_bb.calc_z_delta(new_bb, new_delta.z);
                }
                new_bb += DVec3::new(0.0, 0.0, new_delta.z);

                // Finally clear the cache.
                colliding_bbs.clear();

            });

            let collided_x = delta.x != new_delta.x;
            let collided_z = delta.z != new_delta.z;
            let collided_y = delta.y != new_delta.y;
            let on_ground = self.on_ground || (collided_y && delta.y < 0.0);

            // Handling steps...
            if step_height > 0.0 && on_ground && (sneaking_on_ground || self.step_progress < 0.05) && (collided_x || collided_z) {

                let mut step_delta = delta;
                step_delta.y = step_height as f64;
                let mut step_bb = self.bb;

                COLLIDING_BBS.with_borrow_mut(|colliding_bbs| {

                    debug_assert!(colliding_bbs.is_empty());
                    colliding_bbs.extend(world.iter_hard_boxes_colliding(step_bb + step_delta));
                    
                    // Check collision on Y axis.
                    for colliding_bb in &*colliding_bbs {
                        step_delta.y = colliding_bb.calc_y_delta(step_bb, step_delta.y);
                    }
                    step_bb += DVec3::new(0.0, step_delta.y, 0.0);
            
                    // Check collision on X axis.
                    for colliding_bb in &*colliding_bbs {
                        step_delta.x = colliding_bb.calc_x_delta(step_bb, step_delta.x);
                    }
                    step_bb += DVec3::new(step_delta.x, 0.0, 0.0);
            
                    // Check collision on Z axis.
                    for colliding_bb in &*colliding_bbs {
                        step_delta.z = colliding_bb.calc_z_delta(step_bb, step_delta.z);
                    }
                    step_bb += DVec3::new(0.0, 0.0, step_delta.z);

                    // Check collision on Y axis but in the other direction, to force
                    // the bounding box against the ground.
                    step_delta.y = (-step_height) as f64;
                    for colliding_bb in &*colliding_bbs {
                        step_delta.y = colliding_bb.calc_y_delta(step_bb, step_delta.y);
                    }
                    step_bb += DVec3::new(0.0, step_delta.y, 0.0);

                    // Finally clear the cache.
                    colliding_bbs.clear();
                    
                });

                // Once step delta has been computed, we only use it and its bounding box
                // if the step delta has greater length in horizontal distance.
                if new_delta.x * new_delta.x + new_delta.z * new_delta.z 
                < step_delta.x * step_delta.x + step_delta.z * step_delta.z {
                    
                    new_bb = step_bb;
                    new_delta = step_delta;

                    // PARITY: The notchian implementation fails to get the corect offset
                    // when min.y is negative.
                    let y_offset_from_block = new_bb.min.y.fract();
                    self.step_progress = (self.step_progress as f64 + y_offset_from_block + 0.01) as f32;

                }

            }

            // Now update the position!
            self.pos.x = self.bb.center_x();
            self.pos.y = self.bb.min.y + height_offset as f64 - self.step_progress as f64;
            self.pos.z = self.bb.center_z();

            let collided_x = delta.x != new_delta.x;
            let collided_z = delta.z != new_delta.z;
            self.collided_y = delta.y != new_delta.y;
            self.collided_xz = collided_x || collided_z;
            self.on_ground = self.collided_y && delta.y < 0.0;

            // FIXME: Apparently this part is disabled for MP players.
            if self.on_ground {
                if self.fall_distance > 0.0 {
                    // TODO: Fall damage (depends on actual entity type)
                    self.fall_distance = 0.0;
                }
            } else if new_delta.y < 0.0 {
                self.fall_distance = (self.fall_distance as f64 - new_delta.y) as f32;
            }

            if collided_x {
                self.vel.x = 0.0;
            }

            if collided_y {
                self.vel.y = 0.0;
            }

            if collided_z {
                self.vel.z = 0.0;
            }

            if walk_interact && !sneaking_on_ground /* && this.ridingEntity == null */  { // TODO:

                // Because we are server side, we can change the way the walk distance is
                // calculated. The notchian client or server is triggering one entity 
                // walking on the block, once for every unit of the walk variable, so we
                // can just increase this variable on each

                self.walk_dist = (self.walk_dist as f64 + (new_delta.x * new_delta.x + new_delta.z + new_delta.z).sqrt() * 0.6) as f32;
                let below_pos = self.pos.sub(DVec3::new(0.0, 0.2 + self.step_progress as f64, 0.0)).floor().as_ivec3();

                if self.walk_dist >= 1.0 {
                    if world.walk_block(below_pos, self) {
                        self.walk_dist -= 1.0;
                    }
                }

            }

            // Handle collisions...
            COLLIDING_BLOCKS.with_borrow_mut(|colliding_blocks| {
                
                debug_assert!(colliding_blocks.is_empty());
                for (pos, id, metadata) in world.iter_blocks_in_box(self.bb.inflate(DVec3::splat(0.001))) {
                    if id != block::AIR {
                        colliding_blocks.push((pos, id, metadata));
                    }
                }
                
                for (pos, id, metadata) in colliding_blocks.drain(..) {
                    world.collide_block_unchecked(pos, id, metadata, self);
                }

            });

            // Handle fire...
            let is_wet = self.is_wet(world);
            let mut burning = false;
            for (_, id, _) in world.iter_blocks_in_box(self.bb.inflate(DVec3::splat(-0.001))) {
                if let block::FIRE | block::LAVA_MOVING | block::LAVA_STILL = id {
                    burning = true;
                    break;
                }
            }

            if burning {
                if !self.fire_immune {
                    self.hurt.push(Hurt { damage: 1, origin_id: None });
                }
                if !is_wet {
                    self.fire_time += 1;
                    if self.fire_time == 0 {
                        self.fire_time = 300;
                    }
                }
            } else if self.fire_time <= 0 {
                self.fire_time = -self.fire_resistance;
            }

            if is_wet && self.fire_time > 0 {
                self.fire_time = -self.fire_resistance;
            }
            
        }

    }

    /// Move this entity out of any block it is currently in. This is currently only used
    /// for item entities.
    pub fn move_out_of_block(&mut self, world: &World) {

        // If the item is in an opaque block, move it out of the block.
        // NOTE: The notchian implementation is actually using the middle of the bounding
        // box's Y value, but because we know that this is only used on items, where the
        // position is alredy the middle of the bounding box, then we simplify it here.
        let block_pos = self.pos.floor().as_ivec3();
        if world.is_block_normal_cube(block_pos) {

            let delta = self.pos - block_pos.as_dvec3();

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
                let accel = (self.rand.next_float() * 0.2 + 0.1) as f64;
                if bump_face.is_neg() {
                    self.vel[bump_face.axis_index()] = -accel;
                } else {
                    self.vel[bump_face.axis_index()] = accel;
                }
            }

        }

    }

    pub fn is_wet(&self, world: &World) -> bool {
        self.in_water || world.get_local_weather(self.pos.floor().as_ivec3()) == LocalWeather::Thunder
    }

}




/// Definition of common and heritable function in base entities. We make this to emulate
/// the OOP slop of the original client/server :/
pub trait BaseDef {

    fn base(&self) -> &Base;
    fn base_mut(&mut self) -> &mut Base;

    /// The base ticking method
    fn tick(&mut self, world: &mut World, id: u32) {
        self.tick_(world, id);
    }

    /// Default implementation of [`Self::tick`].
    fn tick_(&mut self, world: &mut World, id: u32) {

        // Handle water
        if let Some(vel) = self.handle_water_vel(world, id) {
            let base = self.base_mut();
            base.vel += vel * 0.014;
            base.in_water = true;
            base.fall_distance = 0.0;
            base.fire_time = 0;
        } else {
            let base = self.base_mut();
            base.in_water = false;
        }
        
        // Handle fire
        let base = self.base_mut();
        if base.fire_time > 0 {
            if base.fire_immune {
                base.fire_time -= 4;
                if base.fire_time < 0 {
                    base.fire_time = 0;
                }
            } else {
                base.fire_time -= 1;
                if (base.fire_time + 1) % 20 == 0 {
                    self.hurt(world, id, 1, HurtReason::Fire);
                }
            }
        }

        // Handle lava
        let base = self.base_mut();
        let lava_bb = base.bb.inflate(DVec3::new(-0.1, -0.4, -0.1));
        base.in_lava = world.iter_blocks_in_box(lava_bb)
            .any(|(_, block, _)| block::material::get_material(block) == Material::Lava);
        if base.in_lava && !base.fire_immune {
            base.fire_time = 600;
        }

        // Handle void
        if base.pos.y < -64.0 {
            self.hurt_void(world, id);
        }

    }

    /// Implement this method to do damages on the entity.
    /// It returns true if the entity has effectively been damaged.
    fn hurt(&mut self, world: &mut World, id: u32, damage: u16, reason: HurtReason) -> bool {
        let _ = (world, id, damage, reason);
        false
    }

    /// Imeplement this method to handle the entity entering the void.
    fn hurt_void(&mut self, world: &mut World, id: u32) {
        world.remove_entity(id, "void");
    }

    /// This function handles water velocity computation for the entity, returning the
    /// velocity if the entity is in water.
    fn handle_water_vel(&mut self, world: &mut World, _id: u32) -> Option<DVec3> {
        let water_bb = self.base().bb.inflate(DVec3::new(-0.001, -0.4 - 0.001, -0.001));
        calc_fluid_vel_in_box(world, water_bb, Material::Water)
    }

}

/// Calculate the velocity of a fluid at given position, this depends on neighbor blocks.
/// This calculation will only take the given material into account, this material should
/// be a fluid material (water/lava), and the given metadata should be the one of the
/// current block the the position.
fn calc_fluid_vel(world: &World, pos: IVec3, material: Material, metadata: u8) -> DVec3 {

    debug_assert!(material.is_fluid());

    let distance = block::fluid::get_actual_distance(metadata);
    let falling = block::fluid::is_falling(metadata);

    let mut vel = DVec3::ZERO;
    let mut down_current = false;

    // Side current...
    for face in Face::HORIZONTAL {

        let face_delta = face.delta();
        let face_pos = pos + face_delta;
        let (face_block, face_metadata) = world.get_block(face_pos).unwrap_or_default();
        let face_material = block::material::get_material(face_block);

        if face_material == material {
            let face_distance = block::fluid::get_actual_distance(face_metadata);
            let delta = face_distance as i32 - distance as i32;
            vel += (face_delta * delta).as_dvec3();
        } else {

            if !face_material.is_solid() {
                let below_pos = face_pos - IVec3::Y;
                let (below_block, below_metadata) = world.get_block(below_pos).unwrap_or_default();
                let below_material = block::material::get_material(below_block);
                if below_material == material {
                    let below_distance = block::fluid::get_actual_distance(below_metadata);
                    let delta = below_distance as i32 - (distance as i32 - 8);
                    vel += (face_delta * delta).as_dvec3();
                }
            }
            
            // If we didn't detect a down current yet, and if the face's material is not
            // the fluid's material, and not ice, then we set the down current if the 
            // face's material is solid
            if falling && !down_current && face_material != Material::Ice && face_material.is_solid() {
                down_current = true;
            }

        }

        // Same as above, but we check the block just above.
        if falling && !down_current {
            let face_up_pos = face_pos + IVec3::Y;
            let (face_up_block, _) = world.get_block(face_up_pos).unwrap_or_default();
            let face_up_material = block::material::get_material(face_up_block);
            if face_up_material != Material::Ice && face_up_material.is_solid() {
                down_current = true;
            }
        }

    }

    if down_current {
        vel = vel.normalize() - DVec3::new(0.0, 6.0, 0.0);
    }

    vel.normalize()

}

/// Calculate, for the given bounding box, the total velocity of the given fluid material.
fn calc_fluid_vel_in_box(world: &World, bb: BoundingBox, material: Material) -> Option<DVec3> {

    let max_y = bb.max.y.add(1.0).floor();
    let mut vel = None::<DVec3>;

    for (pos, block, metadata) in world.iter_blocks_in_box(bb) {
        let pos_material = block::material::get_material(block);
        if pos_material == material {
            let height = block::fluid::get_actual_height(metadata);
            if max_y >= pos.y as f64 + height as f64 {
                vel = Some(vel.unwrap_or_default() + calc_fluid_vel(world, pos, pos_material, metadata));
            }
        }
    }

    vel.map(|vel| vel.normalize_or_zero())

}
