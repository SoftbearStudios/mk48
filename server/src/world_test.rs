use crate::entities::EntityIndex;
use crate::entity::Entity;
use crate::world::World;
use common::altitude::Altitude;
use common::entity::EntityKind;
use common::world::ARCTIC;
use glam::Vec2;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_polygon_mut, Blend};
use imageproc::point::Point;
use maybe_parallel_iterator::IntoMaybeParallelIterator;
use std::sync::Mutex;

impl World {
    /// Creates a false-color CPU rendering of the world for testing purposes.
    pub fn test_render(&self, center: Vec2, radius: f32, resolution: u32) -> RgbaImage {
        let mut canvas = RgbaImage::new(resolution, resolution);

        // Terrain.
        for pixel_y in 0..resolution {
            let fractional_y = 1.0 - (pixel_y as f32 + 0.5) / resolution as f32;
            let position_y = center.y + (fractional_y - 0.5) * 2.0 * radius;
            for pixel_x in 0..resolution {
                let fractional_x = (pixel_x as f32 + 0.5) / resolution as f32;
                let position_x = center.x + (fractional_x - 0.5) * 2.0 * radius;
                let position = Vec2::new(position_x, position_y);
                let color = canvas.get_pixel_mut(pixel_x, pixel_y);

                let altitude = self.terrain.sample(position).unwrap_or(Altitude::ZERO);

                *color = if position.length_squared() > self.radius.powi(2) {
                    Rgba::from([0, 0, 0, 255])
                } else if altitude < Altitude::ZERO {
                    Rgba::from([0, 0, 255, 255])
                } else {
                    if position_y > ARCTIC {
                        Rgba::from([0, 230, 255, 255])
                    } else {
                        Rgba::from([0, 255, 0, 255])
                    }
                };
            }
        }

        let canvas = Mutex::new(Blend(canvas));

        // Entities.
        self.entities
            .par_iter()
            .into_maybe_parallel_iter()
            .for_each(|(_, entity): (EntityIndex, &Entity)| {
                let position = entity.transform.position;
                let normal = entity.transform.direction.to_vec();

                let draw_rect = |margin: f32, color: Rgba<u8>| {
                    let half_length = normal * (entity.data().length + margin);
                    let half_width = normal.perp() * (entity.data().width + margin);

                    let corners = [
                        position + half_length * 0.5 + half_width * 0.5,
                        position - half_length * 0.5 + half_width * 0.5,
                        position - half_length * 0.5 - half_width * 0.5,
                        position + half_length * 0.5 - half_width * 0.5,
                    ];

                    let mut corner_pixels = corners.map(|pos| {
                        // -1 to 1 valid
                        let scaled = (pos - center) / radius;
                        let pixel = ((scaled + 1.0) * resolution as f32 * 0.5).as_ivec2();
                        Point::new(pixel.x, resolution as i32 - pixel.y - 1)
                    });

                    if corner_pixels[0] == corner_pixels[3] {
                        // Don't panic draw_polygon_mut!
                        corner_pixels[0].x -= 1;
                    }

                    draw_polygon_mut(&mut *canvas.lock().unwrap(), &corner_pixels, color);
                };

                draw_rect(0.0, Rgba::from([255, 255, 255, 255]));
                if entity.data().kind == EntityKind::Boat {
                    draw_rect(100.0, Rgba::from([255, 255, 255, 128]));
                }
            });

        canvas.into_inner().unwrap().0
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::AsCommandTrait;
    use crate::world::World;
    use crate::Server;
    use common::entity::{EntityData, EntityType};
    use common::protocol::{Command, Spawn};
    use common::ticks::Ticks;
    use common::util::level_to_score;
    use core_protocol::id::PlayerId;
    use game_server::player::{PlayerData, PlayerTuple};
    use glam::Vec2;
    use rand::prelude::IteratorRandom;
    use rand::{thread_rng, Rng};
    use server_util::generate_id::generate_id;
    use std::sync::Arc;

    #[test]
    fn test_render() {
        test_render_with(0, 256);
        test_render_with(10, 1024);
        test_render_with(50, 1024);
        test_render_with(100, 2048);
        test_render_with(200, 4096);
        test_render_with(500, 4096);
    }

    fn test_render_with(player_count: usize, resolution: u32) {
        crate::noise::init();

        let world_radius =
            World::target_radius(player_count as f32 * 1500f32.powi(2) * std::f32::consts::PI);

        println!("rad: {}", world_radius);

        let mut world = World::new(world_radius);
        let mut rng = thread_rng();

        let players: Vec<Arc<PlayerTuple<Server>>> = (0..player_count)
            .map(|i| {
                Arc::new(PlayerTuple::new(PlayerData::new(
                    if rng.gen() {
                        PlayerId::nth_bot(i).unwrap()
                    } else {
                        PlayerId(generate_id())
                    },
                    None,
                )))
            })
            .collect();

        for _ in 0..100 {
            world.spawn_statics(Ticks::from_whole_secs(10));
        }

        for player in &players {
            let bot = player.borrow_player().is_bot();
            let level = ((rng.gen::<f32>().powi(3) * EntityData::MAX_BOAT_LEVEL as f32) as u8)
                .clamp(1, EntityData::MAX_BOAT_LEVEL);
            let score = level_to_score(level);
            player.borrow_player_mut().score = score;
            let entity_type = EntityType::iter()
                .filter(|t| t.can_spawn_as(score, bot) && t.data().level == level)
                .choose(&mut rng)
                .unwrap();
            let spawn = Command::Spawn(Spawn { entity_type });
            const SPAWN_ATTEMPTS: usize = 25;
            for i in 0..=SPAWN_ATTEMPTS {
                match spawn.as_command().apply(&mut world, player) {
                    Err(e) => {
                        if i == SPAWN_ATTEMPTS || !bot {
                            panic!(
                                "spawn {:?} command by player={} resulted in {}",
                                entity_type, !bot, e
                            );
                        }
                    }
                    Ok(_) => break,
                }
            }
        }

        let image = world.test_render(Vec2::ZERO, world_radius, resolution);
        image
            .save(format!("test_render_{}.png", player_count))
            .unwrap();
    }
}
