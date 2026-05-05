
use glam::DVec3;

use crate::block;
use crate::geom::{BoundingBox, Face};
use crate::world::World;

use super::Entity;


crate::class::class! {
    pub struct LightningBolt: Entity {
        state: i8 = 2,
        fire_count: u8,
    }
}

impl LightningBolt {

    pub fn tick(&mut self, world: &mut World, id: u32) {

        self.super_mut()._tick(world, id);

        self.state = self.state - 1;
        if self.state < 0 {
            if self.fire_count == 0 {
                world.remove_entity(id, "lightning bolt end");
            } else if self.state < -self.rand.next_int_bounded(10) as i8 {
                self.fire_count = self.fire_count - 1;
                self.state = 1;
                let _ = self.rand.next_long();  // Dummy random
                // PARITY: It's checking that there are 10 chunks around...
                let pos = self.pos.floor().as_ivec3();
                if let Some((0, _)) = world.get_block(pos) {
                    if world.can_place_block(pos, Face::NegY, block::FIRE) {
                        world.set_block(pos, block::FIRE, 0);
                    }
                }
            }
        }

        if self.state >= 0 {
            
            let bb = BoundingBox {
                min: self.pos - DVec3::splat(3.0),
                max: self.pos + DVec3::new(3.0, 9.0, 3.0),
            };

            // TODO:
            
            // let entities = world.iter_entities_colliding_mut(bb)
            //     .map(|(entity_id, _)| entity_id)
            //     .collect::<Vec<_>>();



            // for (entity_id, _) in world.iter_entities_colliding_mut(bb) {
            //     entity.struck_by_lightning(world, id);
            // }

        }
        
    }

}
