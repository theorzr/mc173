//! Data structure for storing a world (overworld or nether) at runtime.

use std::cell::Cell;
use std::collections::{HashMap, BTreeSet, HashSet, VecDeque};
use std::collections::hash_map;

use std::iter::FusedIterator;
use std::cmp::Ordering;
use std::hash::Hash;
use std::sync::Arc;
use std::slice;
use std::mem;

use glam::{IVec3, Vec2, DVec3};
use indexmap::IndexMap;

use tracing::trace;

use crate::entity::{Entity, EntityCategory, EntityKind, LightningBolt};
use crate::block_entity::BlockEntity;
use crate::biome::Biome;
use crate::chunk::{Chunk,
    calc_chunk_pos, calc_chunk_pos_unchecked, calc_entity_chunk_pos,
    CHUNK_HEIGHT, CHUNK_WIDTH};

use crate::geom::{BoundingBox, Face};
use crate::java::JavaRandom;
use crate::item::ItemStack;
use crate::block;


// Following modules are order by order of importance, last modules depends on first ones.
pub mod material;
pub mod bound;
pub mod power;
pub mod loot;
pub mod interact;
pub mod place;
pub mod r#break;
pub mod r#use;
pub mod tick;
pub mod notify;
pub mod explode;
pub mod path;


/// The included maximum distance a player can be from a chunk for it to have natural 
/// spawning. This is a square distance, meaning that the player can be at N,N and it
/// will work.
const NATURAL_SPAWN_MAX_DIST: u8 = 8;
/// Same as [`NATURAL_SPAWN_MAX_DIST`] but for random ticks.
const RANDOM_TICK_MAX_DIST: u8 = 9;
/// The number of random ticks to do per chunk.
const RANDOM_TICK_PER_CHUNK: usize = 80;

/// A data-structure that fully describes a Minecraft beta 1.7.3 world, with all its 
/// blocks, lights, biomes, entities and block entities. It also keep the current state
/// of the world such as time and weather and allows ticking it step by step.
/// 
/// # Components 
/// 
/// This data structure stores different kind of component:
/// - Chunks, these are the storage for block, light and height map of a 16x16 column in
///   the world with a height of 128. This component has the largest memory footprint
///   overall and is stored in shared reference to avoid too much memory copy.
///   A chunk must be present in order to set block in the world.
/// - Entities, basically anything that needs to be ticked with 3 dimensional coordinates.
///   They can control their own position, velocity and look for example.
/// - Block Entities, this is a mix between entities and blocks, they can be ticked but
///   are attached to a block position that they cannot control.
/// 
/// These components are independent, but are internally optimized for access. For example
/// entities are not directly linked to a chunk, but an iterator over entities within a
/// chunk can be obtained.
/// 
/// This data structure is however not designed to handle automatic chunk loading and 
/// saving, every chunk needs to be manually inserted and removed, same for entities and
/// block entities.
/// 
/// # Logic
/// 
/// This data structure is also optimized for actually running the world's logic if 
/// needed. Such as weather, random block ticking, scheduled block ticking, entity 
/// ticking or block notifications.
/// 
/// # Events
/// 
/// This structure also allows listening for events within it through a queue of 
/// [`Event`], events listening is disabled by default but can be enabled by swapping
/// a `Vec<Event>` into the world using the [`swap_events`](Self::swap_events). Events 
/// are generated either by world's ticking logic or by manual changes to the world. 
/// Events are ordered chronologically, for example and entity cannot be removed before 
/// being spawned.
/// 
/// # Naming convention
/// 
/// Methods provided on this structure should follow a naming convention depending on the
/// action that will apply to the world:
/// - Methods that don't alter the world and return values should be prefixed by `get_`, 
///   these are getters and should not usually compute too much, getters that returns
///   mutable reference should be suffixed with `_mut`;
/// - Getter methods that return booleans should prefer `can_`, `has_` or `is_` prefixes;
/// - Methods that alter the world by running a logic tick should start with `tick_`;
/// - Methods that iterate over some world objects should start with `iter_`, the return
///   iterator type should preferably be a new type (not `impl Iterator`);
/// - Methods that run on internal events can be prefixed by `handle_`;
/// - All other methods should use a proper verb, preferably composed of one-word to
///   reduce possible meanings (e.g. are `schedule_`, `break_`, `spawn_`, `insert_` or
///   `remove_`).
/// 
/// Various suffixes can be added to methods, depending on the world area affected by the
/// method, for example `_in`, `_in_chunk`, `_in_box` or `_colliding`.
/// Any mutation prefix `_mut` should be placed at the very end.
/// 
/// # Roadmap
/// 
/// - Make a diagram to better explain the world structure with entity caching.
#[derive(Clone)]
pub struct World {
    /// When enabled, this contains the list of events that happened in the world since
    /// it was last swapped. This swap behavior is really useful in order to avoid 
    /// borrowing issues, by temporarily taking ownership of events, the caller can get
    /// a mutable reference to that world at the same time.
    events: Option<Vec<Event>>,
    /// The dimension
    dimension: Dimension,
    /// The world time, increasing on each tick. This is used for day/night cycle but 
    /// also for registering scheduled ticks.
    time: u64,
    /// The world's global random number generator, it is used everywhere to randomize
    /// events in the world, such as plant grow.
    rand: JavaRandom,
    /// The mapping of world chunks, with optional world components linked to them, such
    /// as chunk data, entities and block entities.
    chunks: Vec<ChunkComponent>,
    /// Mapping of chunk position to their 
    chunks_pos_map: HashMap<(i32, i32), usize>,
    /// A cache of the last requested chunk, this allows us to avoid going through the
    /// chunks hash map to fetch the same chunk index over and over (which is likely the
    /// case with path finding).
    chunks_pos_cache: Cell<Option<(i32, i32, usize)>>,
    /// A list of chunks with natural spawn enabled, this is updated when ticking.
    chunks_with_natural_spawn: Vec<usize>,
    /// A list of chunks with random tick enabled, this is updated when ticking.
    chunks_with_random_tick: Vec<usize>,
    /// Total entities count spawned since the world is running. Also used to give 
    /// entities a unique id.
    entities_count: u32,
    /// The internal list of all loaded entities.
    entities: Vec<EntityComponent>,
    /// Entities' index mapping from their unique id.
    entities_id_map: HashMap<u32, usize>,
    /// This index map contains a mapping for every player entity.
    player_entities_map: IndexMap<u32, usize>,
    /// Same as entities but for block entities.
    block_entities: Vec<BlockEntityComponent>,
    /// Mapping of block entities to they block position.
    block_entities_pos_map: HashMap<IVec3, usize>,
    /// Total scheduled ticks count since the world is running.
    block_ticks_count: u64,
    /// Mapping of scheduled ticks in the future.
    block_ticks: BTreeSet<BlockTick>,
    /// A set of all scheduled tick states, used to avoid ticking twice the same position
    /// and block id. 
    block_ticks_states: HashSet<BlockTickState>,
    /// Queue of pending light updates to be processed.
    light_updates: VecDeque<LightUpdate>,
    /// This is the wrapping seed used by random ticks to compute random block positions.
    random_ticks_seed: i32,
    /// The current weather in that world, note that the Notchian server do not work like
    /// this, but rather store two independent state for rain and thunder, but we simplify
    /// the logic in this implementation since it is not strictly needed to be on parity.
    weather: Weather,
    /// Next time when the weather should be recomputed.
    weather_next_time: u64,
    /// The current sky light level, depending on the current time. This value is used
    /// when subtracted from a chunk sky light level.
    sky_light_subtracted: u8,
}

/// Core methods for worlds.
impl World {

    /// Create a new world of the given dimension with no events queue by default, so
    /// events are disabled.
    pub fn new(dimension: Dimension) -> Self {
        Self {
            events: None,
            dimension,
            time: 0,
            rand: JavaRandom::new_seeded(),
            chunks: Vec::new(),
            chunks_pos_map: HashMap::new(),
            chunks_pos_cache: Cell::new(None),
            chunks_with_natural_spawn: Vec::new(),
            chunks_with_random_tick: Vec::new(),
            entities_count: 0,
            entities: Vec::new(),
            entities_id_map: HashMap::new(),
            player_entities_map: IndexMap::new(),
            block_entities: Vec::new(),
            block_entities_pos_map: HashMap::new(),
            block_ticks_count: 0,
            block_ticks: BTreeSet::new(),
            block_ticks_states: HashSet::new(),
            light_updates: VecDeque::new(),
            random_ticks_seed: JavaRandom::new_seeded().next_int(),
            weather: Weather::Clear,
            weather_next_time: 0,
            sky_light_subtracted: 0,
        }
    }

    /// This function can be used to swap in a new events queue and return the previous
    /// one if relevant. Giving *None* events queue disable events registration using
    /// the [`push_event`] method. Swapping out the events is the only way of reading 
    /// them afterward without borrowing the world.
    /// 
    /// [`push_event`]: Self::push_event
    pub fn swap_events(&mut self, events: Option<Vec<Event>>) -> Option<Vec<Event>> {
        mem::replace(&mut self.events, events)
    }

    /// Return true if this world has an internal events queue that enables usage of the
    /// [`push_event`] method.
    /// 
    /// [`push_event`]: Self::push_event
    pub fn has_events(&self) -> bool {
        self.events.is_some()
    }

    /// Push an event in this world. This only actually push the event if events are 
    /// enabled. Events queue can be swapped using [`swap_events`](Self::swap_events) 
    /// method.
    #[inline]
    pub fn push_event(&mut self, event: Event) {
        if let Some(events) = &mut self.events {
            events.push(event);
        }
    }

    /// Get the dimension of this world, this is basically only for sky color on client
    /// and also for celestial angle on the server side for sky light calculation. This
    /// has not direct relation with the actual world generation that is providing this
    /// world with chunks and entities.
    pub fn get_dimension(&self) -> Dimension {
        self.dimension
    }

    /// Get the world time, in ticks.
    pub fn get_time(&self) -> u64 {
        self.time
    }

    /// Get a mutable access to this world's random number generator.
    pub fn get_rand_mut(&mut self) -> &mut JavaRandom {
        &mut self.rand
    }

    // =================== //
    //   CHUNK SNAPSHOTS   //
    // =================== //

    /// Insert a chunk snapshot into this world at its position with all entities and 
    /// block entities attached to it. If some entity/block entity were already present 
    /// in this chunks, the existing ones are preserved.
    pub fn insert_chunk_snapshot(&mut self, snapshot: ChunkSnapshot) {
        
        self.set_chunk(snapshot.cx, snapshot.cz, snapshot.chunk);
        
        for entity in snapshot.entities {
            debug_assert_eq!(calc_entity_chunk_pos(entity.0.pos), (snapshot.cx, snapshot.cz), "incoherent entity in chunk snapshot");
            self.spawn_entity(entity);
        }

        for (pos, block_entity) in snapshot.block_entities {
            debug_assert_eq!(calc_chunk_pos_unchecked(pos), (snapshot.cx, snapshot.cz), "incoherent block entity in chunk snapshot");
            self.set_block_entity(pos, block_entity);
        }

    }

    /// Create a snapshot of a chunk's content, this only works if chunk data is existing.
    pub fn take_chunk_snapshot(&self, cx: i32, cz: i32) -> Option<ChunkSnapshot> {
        let index = self.get_chunk_index(cx, cz)?;
        let comp = &self.chunks[index];
        let chunk = comp.data.as_ref()?;
        Some(ChunkSnapshot {
            cx, 
            cz,
            chunk: Arc::clone(&chunk),
            entities: comp.entities.values()
                // Ignoring entities being updated, silently for now.
                .filter_map(|&index| self.entities.get(index).unwrap().inner.as_ref().map(Arc::clone))
                .collect(),
            block_entities: comp.block_entities.iter()
                .filter_map(|(&pos, &index)| self.block_entities.get(index).unwrap().inner.as_ref()
                    .map(|e| (pos, Arc::clone(e))))
                .collect(),
        })
    }

    /// Remove a chunk at given chunk coordinates and return a snapshot of it. If there
    /// is no chunk at the coordinates but entities or block entities are present, None
    /// is returned but entities and block entities are removed from the world.
    pub fn remove_chunk_snapshot(&mut self, cx: i32, cz: i32) -> Option<ChunkSnapshot> {
        
        let index = self.get_chunk_index(cx, cz)?;
        let comp = self.chunks.swap_remove(index);
        let swapped_index = self.chunks.len();

        // We have to invalidate any cached index to this chunk...
        if let Some((ccx, ccz, _)) = self.chunks_pos_cache.get()
        && (ccx, ccz) == (cx, cz) {
            self.chunks_pos_cache.set(None);
        }

        // We must update all entities and block entities in that chunk to keep their id
        // in sync with the new swapped chunk's index.
        if let Some(swapped_comp) = self.chunks.get_mut(index) {

            let prev_index = self.chunks_pos_map.insert((swapped_comp.cx, swapped_comp.cz), index);
            debug_assert_eq!(prev_index, Some(swapped_index), "swapped chunk is incoherent");

            for &entity_index in swapped_comp.entities.values() {
                let prev_index = mem::replace(&mut self.entities[entity_index].chunk_index, index);
                debug_assert_eq!(prev_index, swapped_index, "entity is incoherent in the swapped chunk");
            }

            for &block_entity_index in swapped_comp.block_entities.values() {
                let prev_index = mem::replace(&mut self.block_entities[block_entity_index].chunk_index, index);
                debug_assert_eq!(prev_index, swapped_index, "entity is incoherent in the swapped chunk");
            }

        }

        // Now that we updated all indices, we remove dump all entities and block 
        // entities, but without updating their chunk (because their chunk index
        // is no longer valid! we just removed the chunk...).
        let mut ret = None;

        let entities = comp.entities.keys()
            .filter_map(|&id| self.remove_entity_inner(id, false, "remove chunk snapshot").unwrap().inner)
            .collect();
        
        let block_entities = comp.block_entities.keys()
            .filter_map(|&pos| self.remove_block_entity_inner(pos, false).unwrap().inner
                .map(|e| (pos, e)))
            .collect();
        
        if let Some(chunk) = comp.data {

            ret = Some(ChunkSnapshot { 
                cx, 
                cz,
                chunk,
                entities,
                block_entities,
            });

            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Remove });

        }

        ret

    }

    // =================== //
    //        CHUNKS       //
    // =================== //

    /// Internal function to return the index of a chunk component. It returns None if
    /// the chunk to not exist.
    fn get_chunk_index(&self, cx: i32, cz: i32) -> Option<usize> {

        if let Some((ccx, ccz, index)) = self.chunks_pos_cache.get()
        && (ccx, ccz) == (cx, cz) {
            return Some(index);
        }

        let index = *self.chunks_pos_map.get(&(cx, cz))?;
        self.chunks_pos_cache.set(Some((cx, cz, index)));
        Some(index)

    }

    /// Internal function to return the index of a chunk component, if the chunk component
    /// does not exist, it is created with nothing in it.
    fn ensure_chunk_index(&mut self, cx: i32, cz: i32) -> usize {

        if let Some((ccx, ccz, index)) = self.chunks_pos_cache.get()
        && (ccx, ccz) == (cx, cz) {
            return index;
        }

        let index = match self.chunks_pos_map.entry((cx, cz)) {
            hash_map::Entry::Occupied(o) => *o.get(),
            hash_map::Entry::Vacant(v) => {
                let index = self.chunks.len();
                self.chunks.push(ChunkComponent {
                    cx,
                    cz,
                    data: None,
                    entities: IndexMap::new(),
                    block_entities: HashMap::new(),
                    natural_spawn_next_time: 0,
                    random_tick_next_time: 0,
                });
                v.insert(index);
                index
            }
        };

        self.chunks_pos_cache.set(Some((cx, cz, index)));
        index

    }

    /// Raw function to add a chunk to the world at the given coordinates. Note that the
    /// given chunk only contains block and light data, so no entity or block entity will
    /// be added by this function.
    /// 
    /// If any chunk is existing at this coordinate, it's just replaced and all entities
    /// and block entities are not touched.
    /// 
    /// Only entities and block entities that are in a chunk will be ticked.
    pub fn set_chunk(&mut self, cx: i32, cz: i32, chunk: Arc<Chunk>) {

        let chunk_index = self.ensure_chunk_index(cx, cz);
        let chunk_comp = &mut self.chunks[chunk_index];

        let was_unloaded = chunk_comp.data.replace(chunk).is_none();
        
        if was_unloaded {
            for &index in chunk_comp.entities.values() {
                self.entities.get_mut(index).unwrap().tick_next_time = 0;
            }
            for &index in chunk_comp.block_entities.values() {
                self.block_entities.get_mut(index).unwrap().tick_next_time = 0;
            }
        }
        
        self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Set });

    }

    /// Return true if a given chunk is present in the world.
    pub fn contains_chunk(&self, cx: i32, cz: i32) -> bool {
        
        let Some(index) = self.get_chunk_index(cx, cz) else {
            return false;
        };

        self.chunks[index].data.is_some()

    }

    /// Get a reference to a chunk, if existing.
    pub fn get_chunk(&self, cx: i32, cz: i32) -> Option<&Chunk> {
        let index = self.get_chunk_index(cx, cz)?;
        self.chunks[index].data.as_deref()
    }

    /// Get a mutable reference to a chunk, if existing.
    pub fn get_chunk_mut(&mut self, cx: i32, cz: i32) -> Option<&mut Chunk> {
        let index = self.get_chunk_index(cx, cz)?;
        self.chunks[index].data.as_mut().map(|c| Arc::make_mut(c))
    }

    /// Remove a chunk that may not exists. Note that this only removed the chunk data,
    /// not its entities and block entities.
    pub fn remove_chunk(&mut self, cx: i32, cz: i32) -> Option<Arc<Chunk>> {

        // Here we don't use the self.chunk_index function because we are invalidating
        // it anyway... We check the cache to invalidate, and take its index.
        let chunk_index = 
            if let Some((ccx, ccz, index)) = self.chunks_pos_cache.get()
            && (ccx, ccz) == (cx, cz) {
                self.chunks_pos_cache.set(None);
                index
            } else {
                *self.chunks_pos_map.get(&(cx, cz))?
            };

        let chunk_comp = &mut self.chunks[chunk_index];
        
        let ret = chunk_comp.data.take();
        
        if ret.is_some() {

            for &index in chunk_comp.entities.values() {
                self.entities[index].tick_next_time = u64::MAX;
            }

            for &index in chunk_comp.block_entities.values() {
                self.block_entities[index].tick_next_time = u64::MAX;
            }

            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Remove });

        }

        ret
        
    }

    // =================== //
    //        BLOCKS       //
    // =================== //

    /// Set block and metadata at given position in the world, if the chunk is not
    /// loaded, none is returned, but if it is existing the previous block and metadata
    /// is returned. This function also push a block change event and update lights
    /// accordingly.
    pub fn set_block(&mut self, pos: IVec3, id: u8, metadata: u8) -> Option<(u8, u8)> {

        let (cx, cz) = calc_chunk_pos(pos)?;
        let chunk = self.get_chunk_mut(cx, cz)?;
        let (prev_id, prev_metadata) = chunk.get_block(pos);
        
        if id != prev_id || metadata != prev_metadata {

            chunk.set_block(pos, id, metadata);
            chunk.recompute_height(pos);

            // Schedule light updates if the block light properties have changed.
            if block::material::get_light_opacity(id) != block::material::get_light_opacity(prev_id)
            || block::material::get_light_emission(id) != block::material::get_light_emission(prev_id) {
                self.schedule_light_update(pos, LightKind::Block);
                self.schedule_light_update(pos, LightKind::Sky);
            }

            self.push_event(Event::Block { 
                pos, 
                inner: BlockEvent::Set {
                    id, 
                    metadata,
                    prev_id, 
                    prev_metadata, 
                } 
            });

            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });

        }

        Some((prev_id, prev_metadata))

    }

    /// Same as the [`set_block`] method, but the previous block and new block are 
    /// notified of that removal and addition.
    /// 
    /// [`set_block`]: Self::set_block
    pub fn set_block_self_notify(&mut self, pos: IVec3, id: u8, metadata: u8) -> Option<(u8, u8)> {
        let (prev_id, prev_metadata) = self.set_block(pos, id, metadata)?;
        self.notify_change_unchecked(pos, prev_id, prev_metadata, id, metadata);
        Some((prev_id, prev_metadata))
    }

    /// Same as the [`set_block_self_notify`] method, but additionally the blocks around 
    /// are notified of that neighbor change.
    /// 
    /// [`set_block_self_notify`]: Self::set_block_self_notify
    pub fn set_block_notify(&mut self, pos: IVec3, id: u8, metadata: u8) -> Option<(u8, u8)> {
        let (prev_id, prev_metadata) = self.set_block_self_notify(pos, id, metadata)?;
        self.notify_blocks_around(pos, id);
        Some((prev_id, prev_metadata))
    }

    /// Get block and metadata at given position in the world, if the chunk is not
    /// loaded, none is returned.
    pub fn get_block(&self, pos: IVec3) -> Option<(u8, u8)> {
        let (cx, cz) = calc_chunk_pos(pos)?;
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_block(pos))
    }

    // =================== //
    //        HEIGHT       //
    // =================== //

    /// Get saved height of a chunk column, Y component is ignored in the position. The
    /// returned height is a signed 32 bit integer, but the possible value is only in 
    /// range 0..=128, but it's easier to deal with `i32` because of vectors.
    pub fn get_height(&self, pos: IVec3) -> Option<i32> {
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_height(pos) as i32)
    }

    // =================== //
    //        LIGHTS       //
    // =================== //

    /// Get light level at the given position, in range 0..16.
    pub fn get_light(&self, mut pos: IVec3) -> Light {
        
        if pos.y > 127 {
            pos.y = 127;
        }

        let mut light = Light {
            block: 0,
            sky: 15,
            sky_real: 0,
        };

        if let Some((cx, cz)) = calc_chunk_pos(pos) {
            if let Some(chunk) = self.get_chunk(cx, cz) {
                light.block = chunk.get_block_light(pos);
                light.sky = chunk.get_sky_light(pos);
            }
        }

        light.sky_real = light.sky.saturating_sub(self.sky_light_subtracted);
        light

    }

    /// Schedule a light update to be processed in a future tick.
    ///  
    /// See [`tick_light`](Self::tick_light).
    pub fn schedule_light_update(&mut self, pos: IVec3, kind: LightKind) {
        self.light_updates.push_back(LightUpdate { 
            kind,
            pos,
            credit: 15,
        });
    }

    /// Get the number of light updates remaining to process.
    #[inline]
    pub fn get_light_update_count(&self) -> usize {
        self.light_updates.len()
    }

    // =================== //
    //        BIOMES       //
    // =================== //

    /// Get the biome at some position (Y component is ignored).
    pub fn get_biome(&self, pos: IVec3) -> Option<Biome> {
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let chunk = self.get_chunk(cx, cz)?;
        Some(chunk.get_biome(pos))
    }

    // =================== //
    //       WEATHER       //
    // =================== //

    /// Get the current weather in the world.
    pub fn get_weather(&self) -> Weather {
        self.weather
    }

    /// Set the current weather in this world. If the weather has changed an event will
    /// be pushed into the events queue.
    pub fn set_weather(&mut self, weather: Weather) {
        if self.weather != weather {
            self.push_event(Event::Weather { prev: self.weather, new: weather });
            self.weather = weather;
        }
    }

    /// Return true if it's raining at the given position.
    pub fn get_local_weather(&mut self, pos: IVec3) -> LocalWeather {

        // Weather is clear, no rain anyway.
        if self.weather == Weather::Clear {
            return LocalWeather::Clear;
        }

        // Unchecked because we don't care of Y. Return false if chunk not found.
        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let Some(chunk) = self.get_chunk(cx, cz) else { 
            return LocalWeather::Clear;
        };

        // If the given position is below height, no rain.
        if pos.y < chunk.get_height(pos) as i32 {
            return LocalWeather::Clear;
        }

        // Last check if that the biome can rain.
        let biome = chunk.get_biome(pos);
        if biome.has_snow() {
            LocalWeather::Snow // FIXME: has_snow only applies to snow ground?
        } else if biome.has_rain() {
            LocalWeather::Rain
        } else {
            LocalWeather::Clear
        }

    }

    // =================== //
    //       ENTITIES      //
    // =================== //

    /// Inner function to used to elide generics.
    fn spawn_entity_inner(&mut self, entity: Arc<Entity>) -> u32 {

        // Get the next unique entity id.
        let id = self.entities_count;
        self.entities_count = self.entities_count.checked_add(1)
            .expect("entity count overflow");

        let kind = entity.kind();
        trace!("spawn entity #{id} ({:?})", kind);

        let (cx, cz) = calc_entity_chunk_pos(entity.0.pos);
        let chunk_index = self.ensure_chunk_index(cx, cz);
        let chunk_comp = &mut self.chunks[chunk_index];

        let entity_index = self.entities.len();
        self.entities.push(EntityComponent {
            inner: Some(entity),
            id,
            chunk_index,
            tick_next_time: if chunk_comp.data.is_some() { self.time } else { u64::MAX },
            kind,
        });

        chunk_comp.entities.insert(id, entity_index);
        self.entities_id_map.insert(id, entity_index);
        
        self.push_event(Event::Entity { id, inner: EntityEvent::Spawn });
        self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });

        id

    }

    /// Spawn an entity in this world, this function gives it a unique id and ensure 
    /// coherency with chunks cache.
    /// 
    /// **This function is legal to call from ticking entities, but such entities will be
    /// ticked only on the next world tick.**
    pub fn spawn_entity(&mut self, entity: impl Into<Arc<Entity>>) -> u32 {
        self.spawn_entity_inner(entity.into())
    }

    /// Return true if an entity is present from its id.
    pub fn contains_entity(&self, id: u32) -> bool {
        self.entities_id_map.contains_key(&id)
    }

    /// Return the number of entities in the world, loaded or not.
    #[inline]
    pub fn get_entity_count(&self) -> usize {
        self.entities.len()
    }

    /// Get a generic entity from its unique id. This generic entity can later be checked
    /// for being of a particular type. None can be returned if no entity is existing for
    /// this id or if the entity is the current entity being updated.
    pub fn get_entity(&self, id: u32) -> Option<&Entity> {
        let index = *self.entities_id_map.get(&id)?;
        self.entities.get(index).unwrap().inner.as_deref()
    }

    /// Get a generic entity from its unique id. This generic entity can later be checked
    /// for being of a particular type. None can be returned if no entity is existing for
    /// this id or if the entity is the current entity being updated.
    pub fn get_entity_mut(&mut self, id: u32) -> Option<&mut Entity> {
        let index = *self.entities_id_map.get(&id)?;
        self.entities.get_mut(index).unwrap().inner.as_mut().map(Arc::make_mut)
    }

    /// Remove an entity with given id, returning some boxed entity is successful. This
    /// returns true if the entity has been successfully removed.
    pub fn remove_entity(&mut self, id: u32, reason: &str) -> bool {
        self.remove_entity_inner(id, true, reason).is_some()
    }

    /// Internal version of [`remove_entity`] that returns the removed component.
    /// 
    /// The caller can specify if this entity's chunk_index is valid and should be used
    /// to remove the entity from the chunk component's mapping.
    /// 
    /// The given reason is only used for log tracing.
    fn remove_entity_inner(&mut self, id: u32, have_chunk: bool, reason: &str) -> Option<EntityComponent> {

        let index = self.entities_id_map.remove(&id)?;

        // Also remove the entity from the player map, if it was.
        self.player_entities_map.shift_remove(&id);
        
        // We always swap remove in the first place, and we keep the index of the entity
        // that was moved around because we must update its index in various places.
        let comp = self.entities.swap_remove(index);
        let swapped_index = self.entities.len();
        trace!("remove entity #{id} ({:?}): {reason}", comp.kind);
        
        self.push_event(Event::Entity { id, inner: EntityEvent::Remove });

        // Directly remove the entity from its chunk if needed.
        if have_chunk {
            let chunk_comp = &mut self.chunks[comp.chunk_index];
            let removed_index = chunk_comp.entities.shift_remove(&id);
            debug_assert_eq!(removed_index, Some(index), "entity is incoherent in its chunk");
            let (cx, cz) = (chunk_comp.cx, chunk_comp.cz);
            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
        }

        // The entity that has been swapped has a new index, so we need to update its
        // index into the chunk cache...
        if let Some(swapped_comp) = self.entities.get(index) {

            let prev_index = self.entities_id_map.insert(swapped_comp.id, index);
            debug_assert_eq!(prev_index, Some(swapped_index), "swapped entity is incoherent");

            // Update the index of the entity within the player map, if it is a player.
            self.player_entities_map.entry(swapped_comp.id).and_modify(|i| *i = index);

            // Either the entity have a valid chunk, or the swapped entity is in another
            // chunk, and so it necessarily have a valid chunk.
            if have_chunk || comp.chunk_index != swapped_comp.chunk_index {
                let swapped_chunk_comp = &mut self.chunks[swapped_comp.chunk_index];
                let removed_index = swapped_chunk_comp.entities.insert(swapped_comp.id, index);
                debug_assert_eq!(removed_index, Some(swapped_index), "swapped entity is incoherent in its chunk");
            }

        }

        Some(comp)

    }
    
    // =================== //
    //   PLAYER ENTITIES   //
    // =================== //

    /// Set an entity that is already existing to be a player entity. Player entities are
    /// used as dynamic anchors in the world that are used for things like natural entity
    /// despawning when players are too far away, or for looking at players.
    /// 
    /// This methods returns true if the property has been successfully set.
    pub fn set_player_entity(&mut self, id: u32, player: bool) -> bool {
        let Some(&index) = self.entities_id_map.get(&id) else { return false };
        if player {
            self.player_entities_map.insert(id, index);
        } else {
            self.player_entities_map.shift_remove(&id);
        }
        true
    }

    /// Returns true if the given entity by its id is a player entity. This also returns
    /// false if the entity isn't existing.
    pub fn is_player_entity(&mut self, id: u32) -> bool {
        self.player_entities_map.contains_key(&id)
    }

    /// Returns the number of player entities in the world, loaded or not.
    #[inline]
    pub fn get_player_entity_count(&self) -> usize {
        self.player_entities_map.len()
    }

    // =================== //
    //   BLOCK ENTITIES    //
    // =================== //

    /// Inner function to used to elide generics.
    fn set_block_entity_inner(&mut self, pos: IVec3, block_entity: Arc<BlockEntity>) {

        trace!("set block entity {pos}");

        let (cx, cz) = calc_chunk_pos_unchecked(pos);
        let chunk_index = self.ensure_chunk_index(cx, cz);
        let chunk_comp = &mut self.chunks[chunk_index];

        // We might replace a block at the same position, in that case there is not much
        // to do... We just disable ticking for the current tick.
        match self.block_entities_pos_map.entry(pos) {
            hash_map::Entry::Occupied(o) => {
                let index = *o.get();
                let comp = &mut self.block_entities[index];
                comp.inner = Some(block_entity);
                comp.tick_next_time = if chunk_comp.data.is_some() { self.time + 1 } else { u64::MAX };
                self.push_event(Event::BlockEntity { pos, inner: BlockEntityEvent::Remove });
            }
            hash_map::Entry::Vacant(v) => {
                let block_entity_index = self.block_entities.len();
                self.block_entities.push(BlockEntityComponent {
                    inner: Some(block_entity),
                    tick_next_time: if chunk_comp.data.is_some() { self.time + 1 } else { u64::MAX },
                    chunk_index,
                    pos,
                });
                let prev_index = chunk_comp.block_entities.insert(pos, block_entity_index);
                debug_assert!(prev_index.is_none());
                v.insert(block_entity_index);
            }
        }

        self.push_event(Event::BlockEntity { pos, inner: BlockEntityEvent::Set });
        self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });

    }

    /// Set the block entity at the given position. If a block entity was already at the
    /// position, it is removed silently.
    /// 
    /// **This function is legal to call from ticking entities, but such entities will be
    /// ticked only on the next world tick.**
    #[inline]
    pub fn set_block_entity(&mut self, pos: IVec3, block_entity: impl Into<Arc<BlockEntity>>) {
        self.set_block_entity_inner(pos, block_entity.into());
    }

    /// Returns true if some block entity is present in the world.
    pub fn contains_block_entity(&self, pos: IVec3) -> bool {
        self.block_entities_pos_map.contains_key(&pos)
    }

    /// Return the number of block entities in the world, loaded or not.
    #[inline]
    pub fn get_block_entity_count(&self) -> usize {
        self.block_entities.len()
    }

    /// Get a block entity from its position.
    pub fn get_block_entity(&self, pos: IVec3) -> Option<&BlockEntity> {
        let index = *self.block_entities_pos_map.get(&pos)?;
        self.block_entities[index].inner.as_deref()
    }

    /// Get a block entity from its position.
    pub fn get_block_entity_mut(&mut self, pos: IVec3) -> Option<&mut BlockEntity> {
        let index = *self.block_entities_pos_map.get(&pos)?;
        self.block_entities[index].inner.as_mut().map(Arc::make_mut)
    }

    /// Remove a block entity from a position. Returning true if successful, in this case
    /// the block entity storage is guaranteed to be freed, but the block entity footprint
    /// in this world will be definitely cleaned after ticking.
    pub fn remove_block_entity(&mut self, pos: IVec3) -> bool {
        self.remove_block_entity_inner(pos, true).is_some()
    }

    /// Internal version of `remove_block_entity` that returns the removed component.
    /// 
    /// The caller can specify if the block entity is known to be in an existing chunk
    /// component, if the caller know that the chunk component is no longer existing,
    /// it avoids panicking.
    fn remove_block_entity_inner(&mut self, pos: IVec3, have_chunk: bool) -> Option<BlockEntityComponent> {
        
        let index = self.block_entities_pos_map.remove(&pos)?;
        trace!("remove block entity {pos}");
        
        let comp = self.block_entities.swap_remove(index);
        let swapped_index = self.block_entities.len();
        debug_assert_eq!(comp.pos, pos, "block entity incoherent position");

        self.push_event(Event::BlockEntity { pos, inner: BlockEntityEvent::Remove });
        
        // Directly remove the block entity from its chunk if needed.
        if have_chunk {
            let chunk_comp = &mut self.chunks[comp.chunk_index];
            let removed_index = chunk_comp.block_entities.remove(&pos);
            debug_assert_eq!(removed_index, Some(index), "block entity is incoherent in its chunk");
            let (cx, cz) = (chunk_comp.cx, chunk_comp.cz);
            self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
        }

        // A block entity has been swapped at the removed index, so we need to update any
        // reference to this block entity.
        if let Some(swapped_comp) = self.block_entities.get(index) {

            let prev_index = self.block_entities_pos_map.insert(swapped_comp.pos, index);
            debug_assert_eq!(prev_index, Some(swapped_index), "swapped block entity is incoherent");
            
            // Either the block entity have a valid chunk, or the swapped entity is in 
            // another chunk, and so it necessarily have a valid chunk.
            if have_chunk || comp.chunk_index != swapped_comp.chunk_index {
                let swapped_chunk_comp = &mut self.chunks[swapped_comp.chunk_index];
                let removed_index = swapped_chunk_comp.block_entities.insert(swapped_comp.pos, index);
                debug_assert_eq!(removed_index, Some(swapped_index), "swapped block entity is incoherent in its chunk");
            }

        }

        Some(comp)

    }

    // =================== //
    //   SCHEDULED TICKS   //
    // =================== //

    /// Schedule a tick update to happen at the given position, for the given block id
    /// and with a given delay in ticks. The block tick is not scheduled if a tick was
    /// already scheduled for that exact block id and position.
    pub fn schedule_block_tick(&mut self, pos: IVec3, id: u8, delay: u64) {

        let uid = self.block_ticks_count;
        self.block_ticks_count = self.block_ticks_count.checked_add(1)
            .expect("scheduled ticks count overflow");

        let state = BlockTickState { pos, id };
        if self.block_ticks_states.insert(state) {
            self.block_ticks.insert(BlockTick { time: self.time + delay, state, uid });
        }

    }

    /// Return the current number of scheduled block ticks waiting.
    #[inline]
    pub fn get_block_tick_count(&self) -> usize {
        self.block_ticks.len()
    }

    // =================== //
    //      ITERATORS      //
    // =================== //

    /// Iterate over all blocks in the given area where max is excluded. Unloaded chunks
    /// are not yielded, so the iterator size cannot be known only from min and max.
    #[inline]
    pub fn iter_blocks_in(&self, min: IVec3, max: IVec3) -> BlocksInIter<'_> {
        BlocksInIter::new(self, min, max)
    }

    /// Iterate over all blocks in the chunk at given coordinates.
    #[inline]
    pub fn iter_blocks_in_chunk(&self, cx: i32, cz: i32) -> BlocksInChunkIter<'_> {
        BlocksInChunkIter::new(self, cx, cz)
    }

    /// Iterate over all block entities in a chunk.
    #[inline]
    pub fn iter_block_entities_in_chunk(&self, cx: i32, cz: i32) -> BlockEntitiesInChunkIter<'_> {
        BlockEntitiesInChunkIter {
            indices: self.get_chunk_index(cx, cz).map(|index| self.chunks[index].block_entities.values()),
            block_entities: &self.block_entities
        }
    }

    // TODO: iter_block_entities_in_chunk_mut

    /// Iterate over all entities in the world.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities(&self) -> EntitiesIter<'_> {
        EntitiesIter(self.entities.iter())
    }

    /// Iterator over all entities in the world through mutable references.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_mut(&mut self) -> EntitiesIterMut<'_> {
        EntitiesIterMut(self.entities.iter_mut())
    }

    /// Iterate over all player entities in the world.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_player_entities(&self) -> PlayerEntitiesIter<'_> {
        PlayerEntitiesIter {
            indices: Some(self.player_entities_map.values()),
            entities: &self.entities,
        }
    }

    /// Iterate over all player entities in the world through mutable references.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_player_entities_mut(&mut self) -> PlayerEntitiesIterMut<'_> {
        PlayerEntitiesIterMut {
            indices: Some(self.player_entities_map.values()),
            entities: &mut self.entities,
            #[cfg(debug_assertions)]
            returned_pointers: HashSet::new(),
        }
    }

    /// Iterate over all entities of the given chunk.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_in_chunk(&self, cx: i32, cz: i32) -> EntitiesInChunkIter<'_> {
        EntitiesInChunkIter {
            indices: self.get_chunk_index(cx, cz).map(|index| self.chunks[index].entities.values()),
            entities: &self.entities,
        }
    }

    /// Iterate over all entities of the given chunk through mutable references.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_in_chunk_mut(&mut self, cx: i32, cz: i32) -> EntitiesInChunkIterMut<'_> {
        EntitiesInChunkIterMut {
            indices: self.get_chunk_index(cx, cz).map(|index| self.chunks[index].entities.values()),
            entities: &mut self.entities,
            #[cfg(debug_assertions)]
            returned_pointers: HashSet::new(),
        }
    }

    /// Iterate over all entities colliding with the given bounding box.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_colliding(&self, bb: BoundingBox) -> EntitiesCollidingIter<'_> {

        // The +/- 2.0 is fairly arbitral, it should be the largest bounding box over
        // all entities.
        let (start_cx, start_cz) = calc_entity_chunk_pos(bb.min - 2.0);
        let (end_cx, end_cz) = calc_entity_chunk_pos(bb.max + 2.0);

        EntitiesCollidingIter {
            chunks: ChunkComponentsIter { 
                chunks: &self.chunks, 
                chunks_pos_map: &self.chunks_pos_map,
                range: ChunkRange::new(start_cx, start_cz, end_cx, end_cz),
            },
            indices: None,
            entities: &self.entities,
            bb,
        }

    }

    /// Iterate over all entities colliding with the given bounding box through mut ref.
    /// *This function can't return the current updated entity.*
    #[inline]
    pub fn iter_entities_colliding_mut(&mut self, bb: BoundingBox) -> EntitiesCollidingIterMut<'_> {
        
        let (start_cx, start_cz) = calc_entity_chunk_pos(bb.min - 2.0);
        let (end_cx, end_cz) = calc_entity_chunk_pos(bb.max + 2.0);

        EntitiesCollidingIterMut {
            chunks: ChunkComponentsIter { 
                chunks: &self.chunks, 
                chunks_pos_map: &self.chunks_pos_map,
                range: ChunkRange::new(start_cx, start_cz, end_cx, end_cz),
            },
            indices: None,
            entities: &mut self.entities,
            bb,
            #[cfg(debug_assertions)]
            returned_pointers: HashSet::new(),
        }

    }

    /// Return true if any entity is colliding the given bounding box. The hard argument
    /// can be set to true in order to only check for "hard" entities, hard entities can
    /// prevent block placements and entity spawning.
    pub fn has_entity_colliding(&self, bb: BoundingBox, hard: bool) -> bool {
        self.iter_entities_colliding(bb)
            .any(|(_, entity)| !hard || entity.kind().is_hard())
    }

    // =================== //
    //       TICKING       //
    // =================== //
    
    /// Tick the world, this ticks all entities.
    /// TODO: Guard this from being called recursively from tick functions.
    pub fn tick(&mut self) {

        if self.time % 20 == 0 {
            // println!("time: {}", self.time);
            // println!("weather: {:?}", self.weather);
            // println!("weather_next_time: {}", self.weather_next_time);
            // println!("sky_light_subtracted: {}", self.sky_light_subtracted);
        }

        // We increment the time here, so we no longer have the same time as any
        // 'spawn_entity' or 'set_block_entity' calls, this allows us to immediately
        // tick entities and block entities after their creation, but it prevents 
        // entities from being ticked immediately if created during the tick loop.
        self.time += 1;

        // TODO: Wake up all sleeping player if day time.

        self.tick_chunks();

        self.tick_weather();

        self.tick_sky_light();
        
        self.tick_natural_spawn();
        self.tick_blocks();
        self.tick_random_blocks();
        
        self.tick_light(1000);

        self.tick_entities();
        self.tick_block_entities();
        
    }

    /// Update the loaded chunks for natural spawning of random ticking.
    #[inline(never)]
    fn tick_chunks(&mut self) {

        let max_dist = u8::max(NATURAL_SPAWN_MAX_DIST, RANDOM_TICK_MAX_DIST) as i32;

        self.chunks_with_natural_spawn.clear();
        self.chunks_with_random_tick.clear();

        let time = self.time;

        for &player_entity_index in self.player_entities_map.values() {
            
            let entity = &self.entities[player_entity_index];
            let chunk_comp = &mut self.chunks[entity.chunk_index];
            let (cx, cz) = (chunk_comp.cx, chunk_comp.cz);

            for dcx in -max_dist..=max_dist {
                for dcz in -max_dist..=max_dist {

                    let cx = cx + dcx;
                    let cz = cz + dcz;

                    // Here we are not using the chunk pos cache because we change from 
                    // one chunk to another with no pattern, we would not benefit from it.
                    let Some(&chunk_index) = self.chunks_pos_map.get(&(cx, cz)) else {
                        continue;
                    };

                    let chunk = &mut self.chunks[chunk_index];
                    if chunk.data.is_none() {
                        continue;
                    }
                    
                    if chunk.natural_spawn_next_time != time
                    && dcx.abs() <= NATURAL_SPAWN_MAX_DIST as i32 {
                        chunk.natural_spawn_next_time = time;
                        self.chunks_with_natural_spawn.push(chunk_index);
                    }
                    
                    if chunk.random_tick_next_time != time
                    && dcx.abs() <= RANDOM_TICK_MAX_DIST as i32 {
                        chunk.random_tick_next_time = time;
                        self.chunks_with_random_tick.push(chunk_index);
                    }

                }
            }

        }

    }

    /// Update current weather in the world.
    #[inline(never)]
    fn tick_weather(&mut self) {

        // No weather in the nether.
        if self.dimension == Dimension::Nether {
            return;
        }

        // When it's time to recompute weather.
        if self.time >= self.weather_next_time {

            // Don't update weather on first world tick.
            if self.weather_next_time != 0 {
                let new_weather = match self.weather {
                    Weather::Clear => self.rand.next_choice(&[Weather::Rain, Weather::Thunder]),
                    _ => self.rand.next_choice(&[self.weather, Weather::Clear]),
                };
                self.set_weather(new_weather);
            }

            let bound = if self.weather == Weather::Clear { 168000 } else { 12000 };
            let delay = self.rand.next_int_bounded(bound) as u64 + 12000;
            self.weather_next_time = self.time + delay;

        }

    }

    /// Do natural animal and mob spawning in the world.
    #[inline(never)]
    fn tick_natural_spawn(&mut self) {

        // TODO: Perform sleep spawning if all players are sleeping!

        /// The minimum distance required from any player entity to spawn.
        const SPAWN_MIN_DIST_SQUARED: f64 = 24.0 * 24.0;

        // Categories of entities to spawn, also used to count how many are currently 
        // loaded in the world. We have 4 slots in this array because there are 4
        // entity categories.
        let mut categories_count = [0; EntityCategory::ALL.len()];

        // Count every entity category.
        for comp in self.entities.iter() {
            if comp.tick_next_time >= self.time {
                if let Some(entity) = comp.inner.as_deref() {
                    categories_count[entity.category() as usize] += 1;
                }
            }
        }

        // Take the chunk list temporarily.
        let chunks = mem::take(&mut self.chunks_with_natural_spawn);

        for category in EntityCategory::ALL {

            let max_world_count = category.natural_spawn_max_world_count();

            // Skip the category if it cannot spawn.
            if max_world_count == 0 {
                continue;
            }
            // Skip the category if it already has enough loaded entities.
            if categories_count[category as usize] > max_world_count * self.chunks.len() / 256 {
                continue;
            }

            for &chunk_index in &chunks {

                // Temporary borrowing of chunk data to query biome and block.
                let chunk_comp = &self.chunks[chunk_index];
                let chunk_data = chunk_comp.data.as_deref().unwrap();
                let (cx, cz) = (chunk_comp.cx, chunk_comp.cz);

                let biome = chunk_data.get_biome(IVec3::ZERO);
                let kinds = biome.natural_entity_kinds(category);

                // Ignore this chunk is its biome cannot spawn any entity.
                if kinds.is_empty() {
                    continue;
                }

                // Next we pick a random spawn position within the chunk and check it.
                let center_pos = IVec3 {
                    x: cx * 16 + self.rand.next_int_bounded(16),
                    y: self.rand.next_int_bounded(128),
                    z: cz * 16 + self.rand.next_int_bounded(16),
                };

                // If the block is not valid to spawn the category in, skip chunk.
                let (block, _) = chunk_data.get_block(center_pos);
                if block::material::get_material(block) != category.natural_spawn_material() {
                    continue;
                }

                let chance_sum = kinds.iter().map(|kind| kind.chance).sum::<u16>();
                let index = self.rand.next_int_bounded(chance_sum as i32) as u16;
                let mut chance_acc = 0;
                let mut kind = kinds[0].kind;

                for test_kind in kinds {
                    chance_acc += test_kind.chance;
                    if index < chance_acc {
                        kind = test_kind.kind;
                        break;
                    }
                }

                // Keep the maximum chunk count to compare with spawn count.
                let max_chunk_count = kind.natural_spawn_max_chunk_count();

                // Keep track of the total number of entity spawned in that chunk.
                let mut spawn_count = 0usize;

                'pack: for _ in 0..3 {

                    let mut spawn_pos = center_pos;

                    'chain: for _ in 0..4 {

                        spawn_pos += IVec3 {
                            x: self.rand.next_int_bounded(6) - self.rand.next_int_bounded(6),
                            y: self.rand.next_int_bounded(1) - self.rand.next_int_bounded(1),
                            z: self.rand.next_int_bounded(6) - self.rand.next_int_bounded(6),
                        };

                        // Preliminary check if the block position is valid.
                        if category == EntityCategory::WaterAnimal {

                            // Water animals can only spawn in liquid.
                            if !self.get_block_material(spawn_pos).is_fluid() {
                                continue;
                            }

                            // Water animals cannot spawn if above block is opaque.
                            if self.is_block_opaque_cube(spawn_pos + IVec3::Y) {
                                continue;
                            }

                        } else {
                            
                            // The 2 block column should not be opaque cube.
                            if self.is_block_opaque_cube(spawn_pos) || self.is_block_opaque_cube(spawn_pos + IVec3::Y) {
                                continue;
                            }

                            // Block below should be opaque.
                            if !self.is_block_opaque_cube(spawn_pos - IVec3::Y) {
                                continue;
                            }

                            // PARITY: We don't do the fluid block check because it would
                            // be redundant with the check in 'can_natural_spawn'.

                        }

                        let spawn_pos = spawn_pos.as_dvec3() + DVec3::new(0.5, 0.0, 0.5);

                        // PARITY: We check that this entity would be in the 128.0 block 
                        // no-despawn range of at least one player. This avoid entities
                        // to be instantly removed after spawning.
                        let mut close_player = false;
                        for (_, Entity(player_base, _)) in self.iter_player_entities() {
                            // If there is a player too close to that spawn point, abort.
                            let player_dist_sq = player_base.pos.distance_squared(spawn_pos);
                            if player_dist_sq < SPAWN_MIN_DIST_SQUARED {
                                continue 'chain;
                            } else if player_dist_sq <= 128.0 * 128.0 {
                                close_player = true;
                            }
                        }

                        // Skip if no player is in range to keep this natural entity.
                        if !close_player {
                            continue;
                        }

                        // TODO: Do not spawn inside spawn chunks

                        let mut entity_arc = kind.new_default(spawn_pos);
                        let entity = Arc::get_mut(&mut entity_arc).unwrap();
                        entity.0.persistent = true;
                        entity.0.look.x = self.rand.next_float() * std::f32::consts::TAU;

                        // Important to init natural spawn before checking if it can spawn
                        // because slime may be resized, so this can change the bb.
                        entity.init_natural_spawn(self);

                        // Skip if the entity cannot be spawned.
                        if !entity.can_natural_spawn(self) {
                            continue;
                        }

                        self.spawn_entity(entity_arc);
                        spawn_count += 1;
                        if spawn_count >= max_chunk_count {
                            break 'pack;
                        }

                    }

                }

            }

        }

        // Restore the chunk list.
        self.chunks_with_natural_spawn = chunks;

    }

    /// Update the sky light value depending on the current time, it is then used to get
    /// the real light value of blocks.
    #[inline(never)]
    fn tick_sky_light(&mut self) {

        let time_wrapped = self.time % 24000;
        let mut half_turn = (time_wrapped as f32 + 1.0) / 24000.0 - 0.25;

        if half_turn < 0.0 {
            half_turn += 1.0;
        } else if half_turn > 1.0 {
            half_turn -= 1.0;
        }

        let celestial_angle = match self.dimension {
            Dimension::Nether => 0.5,
            _ => half_turn + (1.0 - ((half_turn * std::f32::consts::PI).cos() + 1.0) / 2.0 - half_turn) / 3.0,
        };

        let factor = (celestial_angle * std::f32::consts::TAU).cos() * 2.0 + 0.5;
        let factor = factor.clamp(0.0, 1.0);
        let factor = match self.weather {
            Weather::Clear => 1.0,
            Weather::Rain => 0.6875,
            Weather::Thunder => 0.47265625,
        } * factor;

        self.sky_light_subtracted = ((1.0 - factor) * 11.0) as u8;

    }

    /// Internal function to tick the internal scheduler.
    #[inline(never)]
    fn tick_blocks(&mut self) {

        debug_assert_eq!(self.block_ticks.len(), self.block_ticks_states.len());

        // Schedule ticks...
        while let Some(tick) = self.block_ticks.first() {
            if self.time > tick.time {
                // This tick should be activated.
                let tick = self.block_ticks.pop_first().unwrap();
                assert!(self.block_ticks_states.remove(&tick.state));
                // Check coherency of the scheduled tick and current block.
                if let Some((id, metadata)) = self.get_block(tick.state.pos) {
                    if id == tick.state.id {
                        self.tick_block_unchecked(tick.state.pos, id, metadata, false);
                    }
                }
            } else {
                // Our set is ordered by time first, so we break when past current time. 
                break;
            }
        }

    }

    /// Internal function to tick random blocks.
    #[inline(never)]
    fn tick_random_blocks(&mut self) {

        // Take the random tick chunk list temporarily.
        let chunks = mem::take(&mut self.chunks_with_random_tick);

        for &chunk_index in &chunks {
            
            let chunk_comp = &self.chunks[chunk_index];
            let chunk_data = chunk_comp.data.as_deref().unwrap();
            let (cx, cz) = (chunk_comp.cx, chunk_comp.cz);

            // Try to spawn lightning bolt.
            let mut lightning_bolt = None;
            if self.weather == Weather::Thunder && self.rand.next_int_bounded(100000) == 0 {

                self.random_ticks_seed = self.random_ticks_seed
                    .wrapping_mul(3)
                    .wrapping_add(1013904223);

                let rand = self.random_ticks_seed >> 2;
                let x = ((rand >> 0) & 15) as u8;
                let z = ((rand >> 8) & 15) as u8;
                let y = chunk_data.get_height(IVec3::new(x as i32, 0, z as i32));
                let biome = chunk_data.get_biome(IVec3::new(x as i32, y as i32, z as i32));
                if biome.has_rain() && !biome.has_snow() {
                    lightning_bolt = Some((x, y, z));
                }

            }

            // TODO: Random snowing.
            
            // Minecraft run 80 random ticks per tick per chunk.
            let mut random_ticks = [(0, 0, 0, 0, 0); RANDOM_TICK_PER_CHUNK];
            for i in 0..RANDOM_TICK_PER_CHUNK {

                self.random_ticks_seed = self.random_ticks_seed
                    .wrapping_mul(3)
                    .wrapping_add(1013904223);

                let rand = self.random_ticks_seed >> 2;
                let x = ((rand >> 0) & 15) as u8;
                let y = ((rand >> 16) & 127) as u8;
                let z = ((rand >> 8) & 15) as u8;

                let (id, metadata) = chunk_data.get_block(IVec3::new(x as i32, y as i32, z as i32));
                random_ticks[i] = (x, y, z, id, metadata);

            }
            
            // Now that we finished accessing the chunk data directly, we can borrow self.
            let chunk_pos = IVec3::new(cx * CHUNK_WIDTH as i32, 0, cz * CHUNK_WIDTH as i32);
            
            if let Some((x, y, z)) = lightning_bolt {
                let pos = chunk_pos + IVec3::new(x as i32, y as i32, z as i32);
                self.spawn_entity(LightningBolt::new_default(pos.as_dvec3()));
            }

            for (x, y, z, id, metadata) in random_ticks {
                let pos = chunk_pos + IVec3::new(x as i32, y as i32, z as i32);
                self.tick_block_unchecked(pos, id, metadata, true);
            }

        }

        // Restore the chunk list.
        self.chunks_with_random_tick = chunks;

    }

    /// Internal function to tick all entities.
    #[inline(never)]
    fn tick_entities(&mut self) {

        for entity_index in 0..self.entities.len() {

            // If the entities vector has shorten we break.
            let Some(comp) = self.entities.get_mut(entity_index) else {
                break;
            };

            if self.time < comp.tick_next_time {
                continue;
            }

            let mut entity = comp.inner.take()
                .expect("entity should be present here");

            let id = comp.id;
            let prev_chunk_index = comp.chunk_index;
            let (prev_cx, prev_cz) = {
                let chunk_comp = &self.chunks[prev_chunk_index];
                (chunk_comp.cx, chunk_comp.cz)
            };
            Arc::make_mut(&mut entity).tick(&mut *self, id);

            // Check if the entity moved to another chunk after update...
            let (new_cx, new_cz) = calc_entity_chunk_pos(entity.0.pos);
            let mut new_chunk_index = prev_chunk_index;
            if (prev_cx, prev_cz) != (new_cx, new_cz) {
                new_chunk_index = self.ensure_chunk_index(new_cx, new_cz);
            }

            // If the entity removed itself, ignore and continue.
            let comp = match self.entities.get_mut(entity_index) {
                Some(comp) if comp.id == id => comp,
                _ => continue,
            };

            comp.inner = Some(entity);
            comp.tick_next_time = self.time + 1;

            // NOTE: This part is really critical as this ensures Memory Safety
            // in iterators and therefore avoids Undefined Behaviors. Each entity
            // really needs to be in a single chunk at a time.
            if prev_chunk_index != new_chunk_index {

                let removed_index = self.chunks[prev_chunk_index].entities.shift_remove(&id);
                debug_assert_eq!(removed_index, Some(entity_index), "entity is incoherent in its previous chunk");

                // Update the world entity to its new chunk and orphan state.
                let chunk_comp = &mut self.chunks[new_chunk_index];
                comp.chunk_index = new_chunk_index;

                // Insert the entity in its new chunk.
                let insert_success = chunk_comp.entities.insert(id, entity_index).is_none();
                debug_assert!(insert_success, "entity was already present in its new chunk");
                // If the next chunk is not loaded, disable ticking on it.
                if chunk_comp.data.is_none() {
                    comp.tick_next_time = u64::MAX;
                }

                self.push_event(Event::Chunk { cx: prev_cx, cz: prev_cz, inner: ChunkEvent::Dirty });
                self.push_event(Event::Chunk { cx: new_cx, cz: new_cz, inner: ChunkEvent::Dirty });

            }

        }

    }

    #[inline(never)]
    fn tick_block_entities(&mut self) {
        
        for block_entity_index in 0..self.block_entities.len() {
            
            // If the entities vector has shorten we break.
            let Some(comp) = self.block_entities.get_mut(block_entity_index) else {
                break;
            };
            
            if self.time < comp.tick_next_time {
                continue;
            }

            let mut block_entity = comp.inner.take()
                .expect("block entity should be present here");

            let pos = comp.pos;
            Arc::make_mut(&mut block_entity).tick(self, pos);

            // We have to be careful, if the block entity has been replaced by another
            // one, we check that it's the same by checking if it should be updated now.
            let comp = match self.block_entities.get_mut(block_entity_index) {
                Some(comp) if self.time >= comp.tick_next_time => comp,
                _ => continue,
            };

            comp.inner = Some(block_entity);
            comp.tick_next_time = self.time + 1;

        }

    }

    /// Tick pending light updates for a maximum number of light updates.
    #[inline(never)]
    pub fn tick_light(&mut self, limit: usize) {

        // IMPORTANT NOTE: This algorithm is terrible but works, I've been trying to come
        // with a better one but it has been too complicated so far.
        
        for _ in 0..limit {

            let Some(update) = self.light_updates.pop_front() else { break };

            let mut max_face_emission = 0;
            for face in Face::ALL {

                let face_pos = update.pos + face.delta();

                let Some((cx, cz)) = calc_chunk_pos(face_pos) else { continue };
                let Some(chunk) = self.get_chunk_mut(cx, cz) else { continue };

                let face_emission = match update.kind {
                    LightKind::Block => chunk.get_block_light(face_pos),
                    LightKind::Sky => chunk.get_sky_light(face_pos),
                };

                max_face_emission = max_face_emission.max(face_emission);
                if max_face_emission == 15 {
                    break;
                }

            }

            let Some((cx, cz)) = calc_chunk_pos(update.pos) else { continue };
            let Some(chunk) = self.get_chunk_mut(cx, cz) else { continue };

            let (id, _) = chunk.get_block(update.pos);
            let opacity = block::material::get_light_opacity(id).max(1);

            let emission = match update.kind {
                LightKind::Block => block::material::get_light_emission(id),
                LightKind::Sky => {
                    // If the block is above ground, then it has
                    let column_height = chunk.get_height(update.pos) as i32;
                    if update.pos.y >= column_height { 15 } else { 0 }
                }
            };

            let new_light = emission.max(max_face_emission.saturating_sub(opacity));
            let mut changed = false;
            let mut sky_exposed = false;

            match update.kind {
                LightKind::Block => {
                    if chunk.get_block_light(update.pos) != new_light {
                        chunk.set_block_light(update.pos, new_light);
                        changed = true;
                    }
                }
                LightKind::Sky => {
                    if chunk.get_sky_light(update.pos) != new_light {
                        chunk.set_sky_light(update.pos, new_light);
                        changed = true;
                        sky_exposed = emission == 15;
                    }
                }
            }

            if changed {
                self.push_event(Event::Chunk { cx, cz, inner: ChunkEvent::Dirty });
            }

            if changed && update.credit >= 1 {
                for face in Face::ALL {
                    // Do not propagate light upward when the updated block is above 
                    // ground, so all blocks above are also exposed and should already
                    // be at max level.
                    if face == Face::PosY && sky_exposed {
                        continue;
                    }
                    self.light_updates.push_back(LightUpdate {
                        kind: update.kind,
                        pos: update.pos + face.delta(),
                        credit: update.credit - 1,
                    });
                }
            }
            
        }

    }

}


/// Types of dimensions, used for ambient effects in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dimension {
    /// The overworld dimension with a blue sky and day cycles.
    Overworld,
    /// The creepy nether dimension.
    Nether,
}

/// Type of weather currently in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weather {
    /// The weather is clear.
    Clear,
    /// It is raining.
    Rain,
    /// It is thundering.
    Thunder,
}

/// Type of weather at a specific position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocalWeather {
    /// The weather is clear at the position.
    Clear,
    /// It is raining at the position.
    Rain,
    /// It is snowing at the position.
    Snow,
}

/// Light values of a position in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Light {
    /// Block light level.
    pub block: u8,
    /// Sky light level.
    pub sky: u8,
    /// The real sky light level, depending on the time and weather.
    pub sky_real: u8,
}

impl Light {

    /// Calculate the maximum static light level (without time/weather attenuation).
    #[inline]
    pub fn max(self) -> u8 {
        u8::max(self.block, self.sky)
    }

    /// Calculate the maximum real light level (with time/weather attenuation).
    #[inline]
    pub fn max_real(self) -> u8 {
        u8::max(self.block, self.sky_real)
    }

    /// Calculate the block brightness from its light levels.
    #[inline]
    pub fn brightness(self) -> f32 {
        // TODO: In nether, OFFSET is 0.1
        const OFFSET: f32 = 0.05;
        let base = 1.0 - self.max_real() as f32 / 15.0;
        (1.0 - base) * (base * 3.0 + 1.0) * (1.0 - OFFSET) + OFFSET
    }

}

/// Different kind of lights in the word.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LightKind {
    /// Block light level, the light spread in all directions and blocks have a minimum 
    /// opacity of 1 in all directions, each block has its own light emission.
    Block,
    /// Sky light level, same as block light but light do not decrease when going down
    /// and every block above height have is has an emission of 15.
    Sky,
}

/// An event that happened in the world.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// An event with a block.
    Block {
        /// The position of the block.
        pos: IVec3,
        /// Inner block event.
        inner: BlockEvent,
    },
    /// An event with an entity given its id.
    Entity {
        /// The unique id of the entity.
        id: u32,
        /// Inner entity event.
        inner: EntityEvent,
    },
    /// A block entity has been set at this position.
    BlockEntity {
        /// The block entity position.
        pos: IVec3,
        /// Inner block entity event.
        inner: BlockEntityEvent,
    },
    /// A chunk event.
    Chunk {
        /// The chunk X position.
        cx: i32,
        /// The chunk Z position.
        cz: i32,
        /// Inner chunk event.
        inner: ChunkEvent,
    },
    /// The weather in the world has changed.
    Weather {
        /// Previous weather in the world.
        prev: Weather,
        /// New weather in the world.
        new: Weather,
    },
    /// Explode blocks.
    Explode {
        /// Center position of the explosion.
        center: DVec3,
        /// Radius of the explosion around center.
        radius: f32,
    },
    /// An event to debug and spawn block break particles at the given position.
    DebugParticle {
        /// The block position to spawn particles at.
        pos: IVec3,
        /// The block to break at this position.
        block: u8,
    }
}

/// An event with a block.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockEvent {
    /// A block has been changed in the world.
    Set {
        /// The new block id.
        id: u8,
        /// The new block metadata.
        metadata: u8,
        /// Previous block id.
        prev_id: u8,
        /// Previous block metadata.
        prev_metadata: u8,
    },
    /// Play the block activation sound at given position and id/metadata.
    Sound {
        /// Current id of the block.
        id: u8,
        /// Current metadata of the block.
        metadata: u8,
    },
    /// A piston has been extended or retracted at the given position.
    Piston {
        /// Face of this piston.
        face: Face,
        /// True if the piston is extending.
        extending: bool,
    },
    /// A note block is playing its note.
    NoteBlock {
        /// The instrument to play.
        instrument: u8,
        /// The note to play.
        note: u8,
    },
}

/// An event with an entity.
#[derive(Debug, Clone, PartialEq)]
pub enum EntityEvent {
    /// The entity has been spawned. The initial chunk position is given.
    Spawn,
    /// The entity has been removed. The last chunk position is given.
    Remove,
    /// The entity changed its position.
    Position {
        pos: DVec3,
    },
    /// The entity changed its look.
    Look {
        look: Vec2,
    },
    /// The entity changed its velocity.
    Velocity {
        vel: DVec3,
    },
    /// The entity has picked up another entity, such as arrow or item. Note that the
    /// target entity is not removed by this event, it's only a hint that this happened
    /// just before the entity may be removed.
    Pickup {
        /// The id of the picked up entity.
        target_id: u32,
    },
    /// The entity is damaged and the damage animation should be played by frontend.
    Damage,
    /// The entity is dead and the dead animation should be played by frontend.
    Dead,
    /// Some unspecified entity metadata has changed.
    Metadata,
}

/// An event with a block entity.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockEntityEvent {
    /// The block entity has been set at its position.
    Set,
    /// The block entity has been removed at its position.
    Remove,
    /// A block entity have seen some of its stored item stack changed.
    Storage {
        /// The storage targeted by this event.
        storage: BlockEntityStorage,
        /// The next item stack at this index.
        stack: ItemStack,
    },
    /// A block entity has made some progress.
    Progress {
        /// The kind of progress targeted by this event.
        progress: BlockEntityProgress,
        /// Progress value.
        value: u16,
    },
    /// A sign block entity has been modified.
    Sign,
}

/// Represent the storage slot for a block entity.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockEntityStorage {
    /// The storage slot is referencing a classic linear inventory at given index.
    Standard(u8),
    FurnaceInput,
    FurnaceOutput,
    FurnaceFuel,
}

/// Represent the progress update for a block entity.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum BlockEntityProgress {
    FurnaceSmeltTime,
    FurnaceBurnMaxTime,
    FurnaceBurnRemainingTime,
}

/// An event with a chunk.
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkEvent {
    /// The chunk has been set at its position. A chunk may have been replaced at that
    /// position.
    Set,
    /// The chunk has been removed from its position.
    Remove,
    /// Any chunk component (block, light, entity, block entity) has been modified in the
    /// chunk so it's marked dirty.
    Dirty,
}

/// A snapshot contains all of the content within a chunk, block, light, height map,
/// entities and block entities are all included. This structure can be considered as
/// a "view" because the chunk data (around 80 KB) is referenced to with a [`Arc`], that
/// allows either uniquely owning it, or sharing it with a world, which is the case when
/// saving a chunk.
#[derive(Clone)]
pub struct ChunkSnapshot {
    /// The X chunk coordinate.
    pub cx: i32,
    /// The Z chunk coordinate.
    pub cz: i32,
    /// The block, light and height map data of the chunk.
    pub chunk: Arc<Chunk>,
    /// The entities in that chunk, note that entities are not guaranteed to have a 
    /// position that is within chunk boundaries.
    pub entities: Vec<Arc<Entity>>,
    /// Block entities in that chunk, all block entities are mapped to their absolute
    /// coordinates in the world.
    pub block_entities: HashMap<IVec3, Arc<BlockEntity>>,
}

impl ChunkSnapshot {

    /// Create a new empty chunk view of the given coordinates.
    pub fn new(cx: i32, cz: i32) -> Self {
        Self {
            cx,
            cz,
            chunk: Chunk::new(),
            entities: Vec::new(),
            block_entities: HashMap::new(),
        }
    }

}

/// This internal structure is used to keep data associated to a chunk coordinate X/Z.
/// It could store chunk data, entities and block entities when present. If a world chunk
/// does not contain data, it is considered **unloaded**. It is also impossible to get
/// a snapshot of an unloaded chunk.
/// 
/// Entities and block entities in **unloaded** chunks are no longer updated as soon as
/// they enter that unloaded chunk.
/// 
/// Note: cloning a chunk component will also clone the chunk's Arc, therefore the whole
/// chunk content is actually cloned only when written to.
#[derive(Debug, Clone)]
struct ChunkComponent {
    /// The chunk X coordinate where this component is cached.
    cx: i32,
    /// The chunk Z coordinate where this component is cached.
    cz: i32,
    /// Underlying chunk. This is important to understand why the data chunk is stored 
    /// in an Atomically Reference-Counted container: first the chunk structure is large
    /// (around 80 KB) so we want it be stored in heap while the Arc container allows us
    /// to work with the chunk in a Clone-On-Write manner.
    /// 
    /// In normal conditions, this chunk will not be shared and so it could be mutated 
    /// using the [`Arc::get_mut`] method that allows mutating the Arc's value if only
    /// one reference exists. But there are situations when we want to have more 
    /// references to that chunk data, for example when saving the chunk we'll temporarily
    /// create a Arc referencing this chunk and pass it to the threaded loader/saver.
    /// If the chunk is mutated while being saved, we'll just clone it and replace this
    /// Arc with a new one that, by definition, has only one reference, all of this based
    /// on the [`Arc::make_mut`] method. Depending on save being fast or not, this clone
    /// will be more or less likely to happen.
    data: Option<Arc<Chunk>>,
    /// Entities belonging to this chunk.
    entities: IndexMap<u32, usize>,
    /// Block entities belonging to this chunk.
    block_entities: HashMap<IVec3, usize>,
    /// The time this chunk should have natural spawning.
    natural_spawn_next_time: u64,
    /// The time this chunk should have random ticking.
    random_tick_next_time: u64,
}

/// Internal type for storing a world entity and keep track of its current chunk.
#[derive(Debug, Clone)]
struct EntityComponent {
    /// The actual object, it's set to none whenever it's being ticked.
    /// It's stored in an Arc to provide Clone-on-Write when making chunk snapshot.
    inner: Option<Arc<Entity>>,
    /// Unique entity id is duplicated here to allow us to access it event when entity
    /// is updating.
    id: u32,
    /// The chunk index this entity is in.
    chunk_index: usize,
    /// The minimum world time expected before this entity is ticked.
    tick_next_time: u64,
    /// This field describes the initial entity kind of the entity when spawned, it should
    /// not be changed afterward by ticking functions.
    kind: EntityKind,
}

/// Internal type for storing a world block entity.
#[derive(Debug, Clone)]
struct BlockEntityComponent {
    /// The actual object, it's set to none whenever it's being ticked.
    /// It's stored in an Arc to provide Clone-on-Write when making chunk snapshot.
    inner: Option<Arc<BlockEntity>>,
    /// Position of that block entity.
    pos: IVec3,
    /// The chunk index this entity is in.
    chunk_index: usize,
    /// The minimum world time expected before this entity is ticked.
    tick_next_time: u64,
}

/// A block tick position, this is always linked to a [`ScheduledTick`] being added to
/// the tree map, this structure is also stored appart in order to check that two ticks
/// are not scheduled for the same position and block id.
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct BlockTickState {
    /// Position of the block to tick.
    pos: IVec3,
    /// The expected id of the block, if the block has no longer this id, this tick is
    /// ignored.
    id: u8,
}

/// A block tick scheduled in the future, it's associated to a world time in a tree map.
/// This structure is ordered by time and then by position, this allows to have multiple
/// block update at the same time but for different positions.
#[derive(Clone, Eq)]
struct BlockTick {
    /// This tick unique id within the world.
    uid: u64,
    /// The time to tick the block.
    time: u64,
    /// State of that scheduled tick.
    state: BlockTickState,
}

impl PartialEq for BlockTick {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid && self.time == other.time
    }
}

impl PartialOrd for BlockTick {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(Ord::cmp(self, other))
    }
}

impl Ord for BlockTick {
    fn cmp(&self, other: &Self) -> Ordering {
        self.time.cmp(&other.time)
            .then(self.uid.cmp(&other.uid))
    }
}

/// A light update to apply to the world.
#[derive(Clone)]
struct LightUpdate {
    /// Light kind targeted by this update, the update only applies to one of the kind.
    kind: LightKind,
    /// The position of the light update.
    pos: IVec3,
    /// Credit remaining to update light, this is used to limit the number of updates
    /// produced by a block chance initial update. Initial value is something like 15
    /// and decrease for each propagation, when it reaches 0 the light update stops 
    /// propagating.
    credit: u8,
}

/// An iterator for blocks in a world area. 
/// This yields the block position, id and metadata.
pub struct BlocksInIter<'a> {
    /// Back-reference to the containing world.
    world: &'a World,
    /// This contains a temporary reference to the chunk being analyzed. This is used to
    /// avoid repeatedly fetching chunks' map.
    chunk: Option<(i32, i32, Option<&'a Chunk>)>,
    /// Minimum coordinate to fetch.
    start: IVec3,
    /// Maximum coordinate to fetch (exclusive).
    end: IVec3,
    /// Next block to fetch.
    cursor: IVec3,
}

impl<'a> BlocksInIter<'a> {

    #[inline]
    fn new(world: &'a World, mut start: IVec3, mut end: IVec3) -> Self {

        debug_assert!(start.x <= end.x && start.y <= end.y && start.z <= end.z);

        start.y = start.y.clamp(0, CHUNK_HEIGHT as i32 - 1);
        end.y = end.y.clamp(0, CHUNK_HEIGHT as i32 - 1);

        // If one the component is in common, because max is exclusive, there will be no
        // blocks at all to read, so we set max to min so it will directly ends.
        if start.x == end.x || start.y == end.y || start.z == end.z {
            end = start;
        }

        Self {
            world,
            chunk: None,
            start,
            end,
            cursor: start,
        }

    }

}

impl FusedIterator for BlocksInIter<'_> {}
impl Iterator for BlocksInIter<'_> {

    type Item = (IVec3, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            
            // X is the last updated component, so when it reaches max it's done.
            if self.cursor.x == self.end.x {
                break None;
            }

            // We are at the start of a new column, update the chunk.
            if self.cursor.y == self.start.y {
                // NOTE: Unchecked because the Y value is clamped in the constructor.
                let (cx, cz) = calc_chunk_pos_unchecked(self.cursor);
                if !matches!(self.chunk, Some((ccx, ccz, _)) if (ccx, ccz) == (cx, cz)) {
                    self.chunk = Some((cx, cz, self.world.get_chunk(cx, cz)));
                }
            }

            let prev_cursor = self.cursor;

            // This component order is important because it matches the internal layout of
            // chunks, and therefore improve cache efficiency.
            self.cursor.y += 1;
            if self.cursor.y == self.end.y {
                self.cursor.y = self.start.y;
                self.cursor.z += 1;
                if self.cursor.z == self.end.z {
                    self.cursor.z = self.start.z;
                    self.cursor.x += 1;
                }
            }

            // If a chunk exists for the current column.
            if let Some((_, _, Some(chunk))) = self.chunk {
                let (id, metadata) = chunk.get_block(prev_cursor);
                break Some((prev_cursor, id, metadata));
            }

        }
    }

}


/// An iterator for blocks in a world chunk. 
pub struct BlocksInChunkIter<'a> {
    /// Back-reference to the containing world. None if the chunk doesn't exists or the
    /// iterator is exhausted.
    chunk: Option<&'a Chunk>,
    /// Current position that is iterated in the chunk.
    cursor: IVec3,
}

impl<'a> BlocksInChunkIter<'a> {

    #[inline]
    fn new(world: &'a World, cx: i32, cz: i32) -> Self {
        Self {
            chunk: world.get_chunk(cx, cz),
            cursor: IVec3::new(cx * CHUNK_WIDTH as i32, 0, cz * CHUNK_WIDTH as i32),
        }
    }

}

impl FusedIterator for BlocksInChunkIter<'_> {}
impl Iterator for BlocksInChunkIter<'_> {

    type Item = (IVec3, u8, u8);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {

        let (block, metadata) = self.chunk?.get_block(self.cursor);
        let ret = (self.cursor, block, metadata);

        // This component order is important because it matches the internal layout of
        // chunks, and therefore improve cache efficiency. When incrementing component,
        // when we reach the next multiple of 16 (for X/Z), we reset the coordinate.
        self.cursor.y += 1;
        if self.cursor.y >= CHUNK_HEIGHT as i32 {
            self.cursor.y = 0;
            self.cursor.z += 1;
            if self.cursor.z & 0b1111 == 0 {
                self.cursor.z -= 16;
                self.cursor.x += 1;
                if self.cursor.x & 0b1111 == 0 {
                    // X is the last coordinate to be updated, when we reach it then we
                    // set chunk to none because iterator is exhausted.
                    self.chunk = None;
                }
            }
        }

        Some(ret)

    }

}

/// An iterator of block entities within a chunk.
pub struct BlockEntitiesInChunkIter<'a> {
    /// The entities indices, returned indices are unique within the iterator.
    indices: Option<hash_map::Values<'a, IVec3, usize>>,
    /// The block entities.
    block_entities: &'a [BlockEntityComponent],
}

impl<'a> Iterator for BlockEntitiesInChunkIter<'a> {

    type Item = (IVec3, &'a BlockEntity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&index) = self.indices.as_mut()?.next() {
            let comp = self.block_entities.get(index).unwrap();
            if let Some(block_entity) = comp.inner.as_deref() {
                return Some((comp.pos, block_entity));
            }
        }
        None
    }

}

/// An iterator over all entities in the world.
#[derive(Debug)]
pub struct EntitiesIter<'a>(slice::Iter<'a, EntityComponent>);

impl FusedIterator for EntitiesIter<'_> {}
impl<'a> Iterator for EntitiesIter<'a> {
    
    type Item = (u32, &'a Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(comp) = self.0.next() {
            if let Some(entity) = comp.inner.as_deref() {
                return Some((comp.id, entity));
            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // We sub 1 to the lower bound because we might have at most one ticking entity!
        let (lower, upper) = self.0.size_hint();
        (lower.saturating_sub(1), upper)
    }

}

/// An iterator over all entities in the world through mutable references.
#[derive(Debug)]
pub struct EntitiesIterMut<'a>(slice::IterMut<'a, EntityComponent>);

impl FusedIterator for EntitiesIterMut<'_> {}
impl<'a> Iterator for EntitiesIterMut<'a> {

    type Item = (u32, &'a mut Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(comp) = self.0.next() {
            if let Some(entity) = comp.inner.as_mut().map(Arc::make_mut) {
                return Some((comp.id, entity));
            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        // We sub 1 to the lower bound because we might have at most one ticking entity!
        let (lower, upper) = self.0.size_hint();
        (lower.saturating_sub(1), upper)
    }

}

// TODO: we are currently using type alias because the logic is exactly the same and it's
// a pain to implement, maybe just use a wrapper in the future.
/// An iterator of player entities in the world.
pub type PlayerEntitiesIter<'a> = EntitiesInChunkIter<'a>;

/// An iterator of player entities in the world through mutable references.
pub type PlayerEntitiesIterMut<'a> = EntitiesInChunkIterMut<'a>;

/// An iterator of entities within a chunk.
pub struct EntitiesInChunkIter<'a> {
    /// The entities indices, returned indices are unique within the iterator.
    /// Might be none if there was no chunk data.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a [EntityComponent],
}

impl FusedIterator for EntitiesInChunkIter<'_> {}
impl<'a> Iterator for EntitiesInChunkIter<'a> {

    type Item = (u32, &'a Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&index) = self.indices.as_mut()?.next() {
            let comp = self.entities.get(index).unwrap();
            if let Some(entity) = comp.inner.as_deref() {
                return Some((comp.id, entity));
            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(indices) = &self.indices {
            // We sub 1 to the lower bound because we might have at most one ticking entity!
            let (lower, upper) = indices.size_hint();
            (lower.saturating_sub(1), upper)
        } else {
            (0, Some(0))
        }
    }

}

/// An iterator of entities within a chunk through mutable references.
pub struct EntitiesInChunkIterMut<'a> {
    /// The entities indices, returned indices are unique within the iterator.
    /// Might be none if there was no chunk data.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a mut [EntityComponent],
    /// Only used when debug assertions are enabled in order to ensure the safety
    /// of the lifetime transmutation.
    #[cfg(debug_assertions)]
    returned_pointers: HashSet<*mut Entity>,
}

impl FusedIterator for EntitiesInChunkIterMut<'_> {}
impl<'a> Iterator for EntitiesInChunkIterMut<'a> {

    type Item = (u32, &'a mut Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(&index) = self.indices.as_mut()?.next() {
            // We ignore updated entities.
            let comp = self.entities.get_mut(index).unwrap();
            if let Some(entity) = comp.inner.as_mut().map(Arc::make_mut) {

                let entity_ptr = entity as *mut _;
                
                // Only check uniqueness of returned pointer with debug assertions.
                #[cfg(debug_assertions)] {
                    assert!(self.returned_pointers.insert(entity_ptr), "wrong unsafe contract");
                }

                // SAFETY: We know that returned indices are unique because they come from
                // a map iterator that have unique "usize" keys. So each entity will be 
                // accessed and mutated once and in one place only. So we transmute the 
                // lifetime to 'a, instead of using the default `'self`. This is almost 
                // the same as the implementation of mutable slice iterators where we can
                // get mutable references to all slice elements at once.
                let entity = unsafe { &mut *entity_ptr };
                return Some((comp.id, entity));

            }
        }
        None
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(indices) = &self.indices {
            // We sub 1 to the lower bound because we might have at most one ticking entity!
            let (lower, upper) = indices.size_hint();
            (lower.saturating_sub(1), upper)
        } else {
            (0, Some(0))
        }
    }

}

/// An iterator of entities that collide with a bounding box.
pub struct EntitiesCollidingIter<'a> {
    /// Chunk components iter whens indices is exhausted.
    chunks: ChunkComponentsIter<'a>,
    /// The entities indices, returned indices are unique within the iterator.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a [EntityComponent],
    /// Bounding box to check.
    bb: BoundingBox,
}

impl FusedIterator for EntitiesCollidingIter<'_> {}
impl<'a> Iterator for EntitiesCollidingIter<'a> {

    type Item = (u32, &'a Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // LOOP: This loop should not cause infinite iterator because self.indices
        // will eventually be none because it is set to none when it is exhausted. 
        loop {

            if self.indices.is_none() {
                self.indices = Some(self.chunks.next()?.entities.values());
            }

            // If there is no next index, set indices to none and loop over.
            if let Some(&index) = self.indices.as_mut().unwrap().next() {
                let comp = &self.entities[index];
                if let Some(entity) = comp.inner.as_deref() {
                    if entity.0.bb.intersects(self.bb) {
                        return Some((comp.id, entity));
                    }
                }
            } else {
                self.indices = None;
            }

        }
    }

}

/// An iterator of entities that collide with a bounding box through mutable references.
pub struct EntitiesCollidingIterMut<'a> {
    /// Chunk components iter whens indices is exhausted.
    chunks: ChunkComponentsIter<'a>,
    /// The entities indices, returned indices are unique within the iterator.
    indices: Option<indexmap::map::Values<'a, u32, usize>>,
    /// The entities.
    entities: &'a mut [EntityComponent],
    /// Bounding box to check.
    bb: BoundingBox,
    /// Only used when debug assertions are enabled in order to ensure the safety
    /// of the lifetime transmutation.
    #[cfg(debug_assertions)]
    returned_pointers: HashSet<*mut Entity>,
}

impl FusedIterator for EntitiesCollidingIterMut<'_> {}
impl<'a> Iterator for EntitiesCollidingIterMut<'a> {

    type Item = (u32, &'a mut Entity);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // LOOP SAFETY: This loop should not cause infinite iterator because self.indices
        // will eventually be none because it is set to none when it is exhausted.
        loop {

            if self.indices.is_none() {
                self.indices = Some(self.chunks.next()?.entities.values());
            }

            // If there is no next index, set indices to none and loop over.
            if let Some(&index) = self.indices.as_mut().unwrap().next() {
                let comp = &mut self.entities[index];
                if let Some(entity) = comp.inner.as_mut().map(Arc::make_mut) {
                    if entity.0.bb.intersects(self.bb) {

                        #[cfg(debug_assertions)] {
                            assert!(self.returned_pointers.insert(entity), "wrong unsafe contract");
                        }

                        // SAFETY: Read safety note of 'EntitiesInChunkIterMut', however
                        // we have additional constraint, because we iterate different 
                        // index map iterators so we are no longer guaranteed uniqueness
                        // of returned indices. However, our world implementation ensures
                        // that any entity is only present in a single chunk.
                        let entity = unsafe { &mut *(entity as *mut Entity) };
                        return Some((comp.id, entity));
                        
                    }
                }
            } else {
                self.indices = None;
            }

        }
    }

}

/// Internal iterator chunk components in a range.
struct ChunkComponentsIter<'a> {
    /// From the World.
    chunks: &'a [ChunkComponent],
    /// From the World.
    chunks_pos_map: &'a HashMap<(i32, i32), usize>,
    /// The range of chunks to iterate on.
    range: ChunkRange,
}

impl FusedIterator for ChunkComponentsIter<'_> {}
impl<'a> Iterator for ChunkComponentsIter<'a> {

    type Item = &'a ChunkComponent;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some((cx, cz)) = self.range.next() {
            // Note that we are not using the chunk position cache, by using
            // World::get_chunk_index, because we are iterating over chunks and we would
            // not benefit from the cache.
            if let Some(&chunk_index) = self.chunks_pos_map.get(&(cx, cz)) {
                return Some(&self.chunks[chunk_index]);
            }
        }
        None
    }

    // TODO: Size hint

}

/// Internal iterator of chunk coordinates, both start and end are inclusive.
/// This iterator will start by iterating over chunk X, and then chunk Z.
/// TODO: Make the end exclusive, this can simplify the interface.
struct ChunkRange {
    cx: i32,
    cz: i32,
    start_cx: i32,
    end_cx: i32,
    end_cz: i32,
}

impl ChunkRange {

    // Construct a chunk range iterator, note that both start and end are included in the
    // range.
    #[inline]
    fn new(start_cx: i32, start_cz: i32, end_cx: i32, end_cz: i32) -> Self {
        Self {
            cx: start_cx,
            cz: start_cz,
            start_cx,
            end_cx,
            end_cz,
        }
    }

}

impl FusedIterator for ChunkRange {}
impl Iterator for ChunkRange {

    type Item = (i32, i32);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        
        if self.cx > self.end_cx || self.cz > self.end_cz {
            return None;
        }

        let ret = (self.cx, self.cz);

        self.cx += 1;
        if self.cx > self.end_cx {
            self.cx = self.start_cx;
            self.cz += 1;
        }

        Some(ret)

    }

    // TODO: Size hint

}
