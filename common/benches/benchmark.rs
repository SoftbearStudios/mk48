use common::terrain::*;
use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut terrain = Terrain::<ZeroGenerator>::new();
    let chunk = terrain.mut_chunk(ChunkId(0, 0));
    let bytes = chunk.to_bytes();
    c.bench_function("Chunk::to_bytes", |b| b.iter(|| chunk.to_bytes()));
    c.bench_function("Chunk::from_bytes", |b| {
        b.iter(|| Chunk::from_bytes(&bytes))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
