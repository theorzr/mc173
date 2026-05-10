//! Base entity structure.


/// Base entity data and logic.
pub struct Base {
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
    pub width: f32,
    /// The height of the bounding box.
    pub height: f32,
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
    pub fire_resistance: i32,
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
