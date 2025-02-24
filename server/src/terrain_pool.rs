use crate::{noise::noise_generator, world::World};
use common::{
    entity::EntityType,
    terrain::{ChunkId, Coord, Terrain},
};
use kodiak_server::gen_radius;
use kodiak_server::rand::{prelude::IteratorRandom, thread_rng};
use std::{
    collections::VecDeque,
    sync::Mutex,
    time::{Duration, Instant},
};

static POOL: Mutex<(VecDeque<Terrain>, Option<Instant>)> = Mutex::new((VecDeque::new(), None));

/// Gets a terrain that, with any luck, is partially generated already.
pub fn new_terrain() -> Terrain {
    let mut pool = POOL.lock().unwrap();
    while pool.0.len() < 3 {
        pool.0.push_back(Terrain::with_generator(noise_generator));
    }
    let ret = pool.0.pop_front().unwrap();
    println!(
        "new_terrain() -> {} generated chunks",
        ret.generated_chunks()
    );
    ret
}

/// Calling this will help `new_terrain` be more efficient.
pub fn improve_terrain_pool() {
    let mut pool = POOL.lock().unwrap();
    let now = Instant::now();
    if pool.1.map(|next| now < next).unwrap_or(false) {
        return;
    }
    pool.1 = Some(now + Duration::from_millis(200));
    let mut rng = thread_rng();
    let Some(terrain) = pool.0.iter_mut().choose(&mut rng) else {
        return;
    };
    let pos = gen_radius(
        &mut rng,
        2.0 * World::target_radius(10.0 * EntityType::FairmileD.data().visual_area()),
    );
    let Some(coord) = Coord::from_position(pos) else {
        return;
    };
    let _ = terrain.get_chunk(ChunkId::from_coord(coord));
}
