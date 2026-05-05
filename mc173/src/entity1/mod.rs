pub mod item;
pub mod painting;
pub mod lightning_bolt;
pub mod living;

use std::cell::RefCell;
use std::ops::{Add, Sub};

use glam::{DVec3, IVec3};

use crate::world::{LocalWeather, World};
use crate::block::material::Material;
use crate::geom::{BoundingBox, Face};
use crate::java::JavaRandom;
use crate::block;

pub use item::Item;
pub use painting::Painting;
pub use lightning_bolt::LightningBolt;
pub use living::Living;


/// Category of entity enumeration, this defines various common properties for groups of
/// entities, such as natural spawning properties. 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityCategory {
    /// All animal entities.
    Animal = 0,
    /// Water animal entities.
    WaterAnimal = 1,
    /// Mob entities.
    Mob = 2,
    /// All remaining entities.
    Other = 3,
}

impl EntityCategory {

    /// Contains all the entity categories.
    pub const ALL: [Self; 4] = [Self::Animal, Self::WaterAnimal, Self::Mob, Self::Other];
    
    /// Returns the maximum number of entities of this category before preventing more
    /// natural spawning. This number will be multiplied by the number of spawn-able
    /// chunks and then by 256 (16x16 chunks). So this is the maximum count of entities
    /// per 16x16 chunks loaded.
    pub fn natural_spawn_max_world_count(self) -> usize {
        match self {
            EntityCategory::Animal => 15,
            EntityCategory::WaterAnimal => 5,
            EntityCategory::Mob => 70,
            EntityCategory::Other => 0,
        }
    }

    /// Returns the material this entity is able to spawn in, this is a preliminary check.
    pub fn natural_spawn_material(self) -> Material {
        match self {
            EntityCategory::Animal => Material::Air,
            EntityCategory::WaterAnimal => Material::Water,
            EntityCategory::Mob => Material::Air,
            EntityCategory::Other => Material::Air,
        }
    }

}

/// Kind of entity, without actual data. This enumeration can be used to construct a
/// real entity instance with default values, to be modified later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Item,
    Painting,
    Boat,
    Minecart,
    Bobber,
    LightningBolt,
    FallingBlock,
    Tnt,
    Arrow,
    Egg,
    Fireball,
    Snowball,
    Human,
    Ghast,
    Slime,
    Pig,
    Chicken,
    Cow,
    Sheep,
    Squid,
    Wolf,
    Creeper,
    Giant,
    PigZombie,
    Skeleton,
    Spider,
    Zombie,
}

impl EntityKind {

    /// Construct a new entity of this kind!
    pub fn new_default(self) -> Entity {
        match self {
            EntityKind::Item => Item::new_default(),
            EntityKind::Painting => Painting::new_default(),
            EntityKind::Boat => todo!(),
            EntityKind::Minecart => todo!(),
            EntityKind::Bobber => todo!(),
            EntityKind::LightningBolt => LightningBolt::new_default(),
            EntityKind::FallingBlock => todo!(),
            EntityKind::Tnt => todo!(),
            EntityKind::Arrow => todo!(),
            EntityKind::Egg => todo!(),
            EntityKind::Fireball => todo!(),
            EntityKind::Snowball => todo!(),
            EntityKind::Human => living::Human::new_default(),
            EntityKind::Ghast => todo!(),
            EntityKind::Slime => todo!(),
            EntityKind::Pig => todo!(),
            EntityKind::Chicken => todo!(),
            EntityKind::Cow => todo!(),
            EntityKind::Sheep => todo!(),
            EntityKind::Squid => todo!(),
            EntityKind::Wolf => todo!(),
            EntityKind::Creeper => todo!(),
            EntityKind::Giant => todo!(),
            EntityKind::PigZombie => todo!(),
            EntityKind::Skeleton => todo!(),
            EntityKind::Spider => todo!(),
            EntityKind::Zombie => todo!(),
        }
    }

    /// Get the category of this entity kind.
    pub fn category(self) -> EntityCategory {
        match self {
            EntityKind::Pig |
            EntityKind::Chicken |
            EntityKind::Cow |
            EntityKind::Sheep |
            EntityKind::Wolf => EntityCategory::Animal,
            EntityKind::Squid => EntityCategory::WaterAnimal,
            EntityKind::Creeper |
            EntityKind::Giant |
            EntityKind::PigZombie |
            EntityKind::Skeleton |
            EntityKind::Spider |
            EntityKind::Zombie |
            EntityKind::Slime => EntityCategory::Mob,
            _ => EntityCategory::Other
        }
    }

    /// Returns the maximum number of entities of that kind that can be spawned at once
    /// when natural spawning in a single chunk.
    pub fn natural_spawn_max_chunk_count(self) -> usize {
        match self {
            EntityKind::Ghast => 1,
            EntityKind::Wolf => 8,
            _ => 4,
        }
    }

}

crate::class::class! {
    /// Base entity data and logic.
    pub struct Entity {
        /// Tell if this entity is persistent or not. A persistent entity is saved with 
        /// its chunk, but non-persistent entities are no saved. For example, all player 
        /// entities are typically non-persistent because these are not real entities. 
        /// Some entities cannot be persistent as they are not supported by the Notchian 
        /// serialization.
        pub persistent: bool,
        /// True if this entity has a hard bounding box, which acts like a real block 
        /// regarding the collision computing when moving of entities (on grounds...).
        pub hard: bool,
        /// True if this entity present natural spawning of entities.
        pub prevent_spawning: bool,
        /// The width of the bounding box.
        pub width: f32 = 0.6,
        /// The height of the bounding box.
        pub height: f32 = 1.8,
        /// The offset of the Y position from the bottom of the bounding.
        pub height_offset: f32,
        /// The bounding box is defining the actual position from the size of the entity, 
        /// the actual position of the entity is derived from it. This is recomputed with
        /// the size by `tick_base` method when entity isn't coherent.
        pub bb: BoundingBox,
        /// The current entity position, it is derived from the bounding box and size, it 
        /// can be forced by setting it and then calling `resize` on entity.
        pub pos: DVec3,
        /// True if an entity pos event should be sent after update.
        /// The current entity velocity.
        pub vel: DVec3,
        /// The maximum height this entity can take as a step.
        pub step_height: f32,
        /// When a step is taken by the entity, the bounding box is immediately pushed 
        /// upward, but the position of the entity is instead more progressive toward 
        /// this new pos.
        pub step_progress: f32,
        /// The yaw angle for the entity. This is in radians with no range guarantee.
        /// Zero when pointing toward PosZ, and then rotate clockwise to NegX, NegZ and 
        /// then PosX.
        pub yaw: f32,
        /// The pitch angle for the entity. This is in radians with no range guarantee.
        /// +pi is toward PosY, and -pi is pointing toward NegY.
        pub pitch: f32,
        /// Lifetime of the entity since it was spawned in the world, it increase at every
        /// world tick.
        pub lifetime: u32,
        /// No clip is used to disable collision check when moving the entity, if no clip 
        /// is false, then the entity will be constrained by bounding box in its way.
        pub no_clip: bool,
        /// True if the entity collided in X and/or Z in the last move.
        pub collided_xz: bool,
        /// True if the entity collided in Y in the last move.
        pub collided_y: bool,
        /// Walk distance, scaled to some factor, when we increment the unit, at least, 
        /// we'll interact with the environment. On each tick, if >= 1, 1 is subtracted 
        /// and a walking is triggered on the block below.
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
        pub fire_resistance: i32 = 1,
        /// True if this entity is immune to fire.
        pub fire_immune: bool,
        /// Remaining air ticks to breathe.
        pub air_time: u32,
        /// If this entity is ridden, this contains its rider entity.
        pub rider_id: Option<u32>,
        /// If this entity is riding, this contains its entity being ridden.
        pub ridden_id: Option<u32>,
        /// If this entity has thrown a bobber for fishing, this contains its entity id.
        pub bobber_id: Option<u32>,
        /// The random number generator used for this entity.
        pub rand: JavaRandom,
        ..{ Item, Painting, LightningBolt, Living }
    }
}

impl Entity {

    /// Get the kind of this entity.
    pub fn kind(&self) -> EntityKind {
        match self.downcast_ref() {
            EntityRef::None(_) => unreachable!(),
            EntityRef::Item(_) => EntityKind::Item,
            EntityRef::Painting(_) => EntityKind::Painting,
            EntityRef::LightningBolt(_) => EntityKind::LightningBolt,
            EntityRef::Living(living) => living.kind(),
        }
    }

    /// Move the entity at the given position.
    pub fn set_pos(&mut self, pos: DVec3) {
        if let EntityMut::Painting(painting) = self.downcast_mut() {
            painting.set_pos(pos.floor().as_ivec3());
        } else {
            let half_width = self.width / 2.0;
            self.pos = pos;
            self.bb.min = pos - DVec3::new(half_width as f64, self.height_offset as f64 + self.step_progress as f64, half_width as f64);
            self.bb.max = pos + DVec3::new(half_width as f64, -self.bb.min.y + self.height as f64, half_width as f64);
        }
    }

    /// Check if the entity can currently naturally spawn at the given position.
    /// 
    /// Note that the Notchian implementation is only implementing this on the Living
    /// entity, but this
    pub fn can_natural_spawn(&self, world: &World) -> bool {
        match self.downcast_ref() {
            EntityRef::Living(living) => living.can_natural_spawn(world),
            _ => false,
        }
    }

    /// Initial the entity if it has been naturally spawned.
    pub fn init_natural_spawn(&mut self, world: &mut World, id: u32) {
        match self.downcast_mut() {
            EntityMut::Living(living) => living.init_natural_spawn(world, id),
            _ => ()
        }
    }

    /// General function tick the entity.
    pub fn tick(&mut self, world: &mut World, id: u32) {
        match self.downcast_mut() {
            EntityMut::None(_) => self._tick(world, id),
            EntityMut::Item(item) => item.tick(world, id),
            EntityMut::Painting(painting) => painting.tick(world, id),
            EntityMut::LightningBolt(_) => todo!(),
            EntityMut::Living(living) => living.tick(world, id),
        }
    }

    fn _tick(&mut self, world: &mut World, id: u32) {

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
            self.void(world, id);
        }

    }

    pub fn move_by(&mut self, world: &mut World, id: u32, delta: DVec3) {
        match self.downcast_mut() {
            EntityMut::Painting(painting) => painting.move_by(world, id, delta),
            _ => self._move_by(world, id, delta),
        }
    }

    /// Move the entity by checking its collisions (or ignoring if no clip).
    fn _move_by(&mut self, world: &mut World, id: u32, delta: DVec3) {

        // We use a thread local for the bounding box vector.
        thread_local! {
            static COLLIDING_BBS: RefCell<Vec<BoundingBox>> = const { RefCell::new(Vec::new()) };
            static COLLIDING_BLOCKS: RefCell<Vec<(IVec3, u8, u8)>> = const { RefCell::new(Vec::new()) };
        }

        // Is this entity interacting with block it walks on...
        let walk_interact = match self.downcast_ref() {
            EntityRef::Item(_) => false,
            // TODO: Boat
            // TODO: Falling sand
            // TODO: Minecart
            // TODO: Spider
            // TODO: Tnt
            // TODO: Wolf
            _ => true
        };

        // If this entity seeing all entities has hard bounding boxes.
        let force_hard = match self.downcast_ref() {
            // TODO: Boat
            // TODO: Minecart
            _ => false,
        };

        let mut delta = delta;

        if self.no_clip {
            self.bb += delta;
            self.pos = DVec3 {
                x: self.bb.center_x(),
                y: self.bb.min.y - self.height_offset as f64,
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
                
                while delta.x != 0.0 && world.iter_hard_boxes_colliding(self.bb + DVec3::new(delta.x, -1.0, 0.0), force_hard).next().is_none() {
                    if delta.x < sneaking_offset && delta.x >= -sneaking_offset {
                        delta.x = 0.0;
                    } else if delta.x > 0.0 {
                        delta.x -= sneaking_offset;
                    } else {
                        delta.x += sneaking_offset;
                    }
                }
                
                while delta.z != 0.0 && world.iter_hard_boxes_colliding(self.bb + DVec3::new(0.0, -1.0, delta.z), force_hard).next().is_none() {
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
                colliding_bbs.extend(world.iter_hard_boxes_colliding(new_bb + new_delta, force_hard));
                
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
            if self.step_height > 0.0 && on_ground && (sneaking_on_ground || self.step_progress < 0.05) && (collided_x || collided_z) {

                let mut step_delta = delta;
                step_delta.y = self.step_height as f64;
                let mut step_bb = self.bb;

                COLLIDING_BBS.with_borrow_mut(|colliding_bbs| {

                    debug_assert!(colliding_bbs.is_empty());
                    colliding_bbs.extend(world.iter_hard_boxes_colliding(step_bb + step_delta, force_hard));
                    
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
                    step_delta.y = (-self.step_height) as f64;
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

                    // PARITY: The notchian implementation fails to get the correct offset
                    // when min.y is negative.
                    let y_offset_from_block = new_bb.min.y.fract();
                    self.step_progress = (self.step_progress as f64 + y_offset_from_block + 0.01) as f32;

                }

            }

            // Now update the position!
            self.pos.x = self.bb.center_x();
            self.pos.y = self.bb.min.y + self.height_offset as f64 - self.step_progress as f64;
            self.pos.z = self.bb.center_z();

            let collided_x = delta.x != new_delta.x;
            let collided_z = delta.z != new_delta.z;
            self.collided_y = delta.y != new_delta.y;
            self.collided_xz = collided_x || collided_z;
            self.on_ground = self.collided_y && delta.y < 0.0;

            // FIXME: Apparently self part is disabled for MP players.
            if self.on_ground {
                if self.fall_distance > 0.0 {
                    self.fall(world, id, self.fall_distance);
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

            if walk_interact && !sneaking_on_ground /* && self.ridingEntity == null */  { // TODO:

                // Because we are server side, we can change the way the walk distance is
                // calculated. The notchian client or server is triggering one entity 
                // walking on the block, once for every unit of the walk variable, so we
                // can just increase self variable on each

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
                
                let entity_id = id;
                for (pos, id, metadata) in colliding_blocks.drain(..) {
                    world.collide_block_unchecked(pos, id, metadata, self, entity_id);
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
                self.hurt_fire(world, id, 1);
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
    
    /// Add the given velocity to the current entity velocity.
    pub fn accel_by(&mut self, world: &mut World, id: u32, vel: DVec3) {
        match self.downcast_mut() {
            EntityMut::Painting(painting) => painting.accel_by(world, id, vel),
            _ => self._accel_by(world, id, vel),
        }
    }
    
    fn _accel_by(&mut self, _world: &mut World, _id: u32, vel: DVec3) {
        self.vel += vel;
    }

    pub fn handle_water_vel(&mut self, world: &mut World, id: u32) -> Option<DVec3> {
        match self.downcast_mut() {
            _ => self._handle_water_vel(world, id),
        }
    }

    fn _handle_water_vel(&mut self, world: &mut World, _id: u32) -> Option<DVec3> {
        let water_bb = self.bb.inflate(DVec3::new(-0.001, -0.4 - 0.001, -0.001));
        calc_fluid_vel_in_box(world, water_bb, Material::Water)
    }

    pub fn hurt(&mut self, world: &mut World, id: u32, damage: u16, reason: HurtReason) -> bool {
        match self.downcast_mut() {
            _ => self._hurt(world, id, damage, reason),
        }
    }

    fn _hurt(&mut self, _world: &mut World, _id: u32, _damage: u16, _reason: HurtReason) -> bool {
        false
    }

    pub fn hurt_fire(&mut self, world: &mut World, id: u32, damage: u16) {
        if !self.fire_immune {
            self.hurt(world, id, damage, HurtReason::Fire);
        }
    }

    pub fn void(&mut self, world: &mut World, id: u32) {
        match self.downcast_mut() {
            _ => self._void(world, id),
        }
    }

    fn _void(&mut self, world: &mut World, id: u32) {
        world.remove_entity(id, "void");
    }

    pub fn fall(&mut self, world: &mut World, id: u32, distance: f32) {
        match self.downcast_mut() {
            _ => self._fall(world, id, distance),
        }
    }

    fn _fall(&mut self, world: &mut World, id: u32, distance: f32) {
        // TODO: make the riding entity fall
    }

    /// 
    pub fn struck_by_lightning(&mut self, world: &mut World, id: u32) {
        match self.downcast_mut() {
            // TODO: Creeper
            // TODO: Pig
            _ => self._struck_by_lightning(world, id),
        }
    }

    fn _struck_by_lightning(&mut self, world: &mut World, id: u32) {
        self.hurt_fire(world, id, 5);
        self.fire_time += 1;
        if self.fire_time == 0 {
            self.fire_time = 300;
        }
    }

    /// Move this entity out of any block it is currently in. This is currently only used
    /// for item entities.
    fn move_out_of_block(&mut self, world: &World) {

        // If the item is in an opaque block, move it out of the block.
        // NOTE: The notchian implementation is actually using the middle of the bounding
        // box's Y value, but because we know that this is only used on items, where the
        // position is already the middle of the bounding box, then we simplify it here.
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

    fn is_wet(&self, world: &World) -> bool {
        self.in_water || world.get_local_weather(self.pos.floor().as_ivec3()) == LocalWeather::Thunder
    }

}

/// The different reasons for hurting an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HurtReason {
    Fire,
    Void,
    Cactus,
    Explosion,
    Entity(u32),
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

/// Check if the given bounding box is inside a given fluid material.
fn is_fluid_in_box(world: &World, bb: BoundingBox, material: Material) -> bool {
    for (pos, id, metadata) in world.iter_blocks_in_box(bb) {
        if block::material::get_material(id) == material {
            let y = pos.y as f64 + block::fluid::get_full_height(metadata) as f64;
            if y >= bb.min.y {
                return true;
            }
        }
    }
    false
}
