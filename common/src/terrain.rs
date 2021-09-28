// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::altitude::Altitude;
use crate::protocol::SerializedChunk;
use crate::protocol::TerrainUpdate;
use crate::transform::DimensionTransform;
use crate::util::lerp;
use fast_hilbert as hilbert;
use glam::Vec2;
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::fmt;
use std::fmt::{Debug, Formatter};
use std::mem::{size_of, transmute};
use std::ops::{Add, Mul, RangeInclusive, Sub};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Coord(pub usize, pub usize);

// Scale of terrain aka meters per pixel.
pub const SCALE: f32 = 25.0;
// Size of whole terrain.
// Must be a power of 2.
pub const SIZE: usize = 1 << 10;
// Offset to convert between signed coordinates to unsigned.
const OFFSET: isize = (SIZE / 2) as isize;

// Size of a chunk.
// Must be a power of 2.
const CHUNK_SIZE: usize = 1 << 6;
// Offset to convert between signed chunk coordinates to unsigned.
const CHUNK_OFFSET: isize = (SIZE / CHUNK_SIZE / 2) as isize;
// Size of terrain in chunks.
const SIZE_CHUNKS: usize = SIZE / CHUNK_SIZE;

pub const SAND_LEVEL: Altitude = Altitude(0);
pub const GRASS_LEVEL: Altitude = Altitude(1 << 4);

/// Terrain data to altitude (non-linear, to allow both shallow and deep areas).
const ALTITUDE_LUT: [i8; 17] = [
    i8::MIN,
    -120,
    -100,
    -50,
    -5,
    -3,
    -2,
    -1,
    0,
    1,
    2,
    4,
    8,
    50,
    100,
    120,
    i8::MAX,
];

/// Converts terrain data into `Altitude`.
const fn lookup_altitude(data: u8) -> Altitude {
    // Linearly interpolate between adjacent altitudes.
    let low = ALTITUDE_LUT[(data >> 4) as usize] as i16;
    let high = ALTITUDE_LUT[((data >> 4) + 1) as usize] as i16;
    let frac = (data & 0b1111) as i16;
    Altitude((low + (high - low) * frac / 0b1111) as i8)
}

/// Any terrain pixel can be represented as a `Coord`.
impl Coord {
    pub fn from_position(v: Vec2) -> Option<Self> {
        let v = v.mul(1.0 / SCALE);
        Self::from_scaled_position(v)
    }

    /// Converts a position to the nearest valid `Coord`.
    fn saturating_from_position(mut pos: Vec2) -> Self {
        pos *= 1.0 / (SCALE as f32);
        pos += OFFSET as f32;
        let x = (pos.x as i64).clamp(0, (SIZE - 1) as i64) as usize;
        let y = (pos.y as i64).clamp(0, (SIZE - 1) as i64) as usize;
        Self(x, y)
    }

    fn from_scaled_position(v: Vec2) -> Option<Self> {
        let coord = unsafe {
            Self(
                (v.x.to_int_unchecked::<isize>() + OFFSET) as usize,
                (v.y.to_int_unchecked::<isize>() + OFFSET) as usize,
            )
        };

        if coord.0 >= SIZE || coord.1 >= SIZE {
            None
        } else {
            Some(coord)
        }
    }

    pub fn corner(&self) -> Vec2 {
        Vec2::new(
            (self.0 as isize - OFFSET) as f32,
            (self.1 as isize - OFFSET) as f32,
        )
        .mul(SCALE)
    }

    /*
    pub fn center(&self) -> Vec2 {
        Vec2::new(
            (self.0 as isize - OFFSET) as f32,
            (self.1 as isize - OFFSET) as f32,
        )
        .add(0.5)
        .mul(SCALE)
    }
     */
}

impl<U> From<(U, U)> for Coord
where
    U: Into<u64>,
{
    fn from(x: (U, U)) -> Self {
        Self(x.0.into() as usize, x.1.into() as usize)
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ChunkId(pub u16, pub u16);

/// Any terrain chunk can be represented as a `ChunkId`.
impl ChunkId {
    fn as_index(&self) -> usize {
        self.0 as usize + self.1 as usize * SIZE_CHUNKS
    }

    fn from_index(index: usize) -> Self {
        Self((index % SIZE_CHUNKS) as u16, (index / SIZE_CHUNKS) as u16)
    }

    fn as_coord(&self) -> Coord {
        Coord(self.0 as usize * CHUNK_SIZE, self.1 as usize * CHUNK_SIZE)
    }

    #[inline]
    fn from_coord(coord: Coord) -> Self {
        Self((coord.0 / CHUNK_SIZE) as u16, (coord.1 / CHUNK_SIZE) as u16)
    }

    fn as_position(&self) -> Vec2 {
        Vec2::new(
            (self.0 as isize - CHUNK_OFFSET) as f32,
            (self.1 as isize - CHUNK_OFFSET) as f32,
        )
        .add(0.5)
        .mul(SCALE * CHUNK_SIZE as f32)
    }

    fn in_radius(&self, position: Vec2, radius: f32) -> bool {
        const HALF: f32 = (SCALE * CHUNK_SIZE as f32) / 2.0;
        let abs_diff = self.as_position().sub(position).abs();
        if abs_diff.x > HALF + radius || abs_diff.y > HALF + radius {
            false
        } else if abs_diff.x <= HALF || abs_diff.y <= HALF {
            true
        } else {
            abs_diff.sub(HALF).max(Vec2::ZERO).length_squared() < radius.powi(2)
        }
    }

    fn saturating_from(mut pos: Vec2) -> Self {
        pos *= 1.0 / (SCALE * CHUNK_SIZE as f32);
        pos += SIZE_CHUNKS as f32 / 2.0;
        let x = (pos.x as i32).clamp(0, (SIZE_CHUNKS - 1) as i32) as u16;
        let y = (pos.y as i32).clamp(0, (SIZE_CHUNKS - 1) as i32) as u16;
        Self(x, y)
    }
}

impl TryFrom<Vec2> for ChunkId {
    type Error = &'static str;

    fn try_from(mut pos: Vec2) -> Result<Self, Self::Error> {
        pos *= 1.0 / (SCALE * CHUNK_SIZE as f32);
        pos += SIZE as f32 / 2.0;
        let (x, y) = (pos.x as i32, pos.y as i32);
        const RANGE: RangeInclusive<i32> = 0..=((SIZE_CHUNKS - 1) as i32);
        if RANGE.contains(&x) && RANGE.contains(&y) {
            Ok(Self(x as u16, y as u16))
        } else {
            Err("out of terrain")
        }
    }
}

type Generator = fn(usize, usize) -> u8;

/// Always returns zero. For placeholder purposes, or when no generator is required.
fn zero_generator(_: usize, _: usize) -> u8 {
    0
}

/// Terrain stores a bitmap representing the altitude at each pixel in a 2D grid.
pub struct Terrain {
    chunks: [[Option<Box<Chunk>>; SIZE_CHUNKS]; SIZE_CHUNKS],
    /// Which chunks were modified since the last reset.
    pub updated: ChunkSet,
    /// Guards chunk generation.
    mutex: Mutex<()>,
    generator: Generator,
}

impl Terrain {
    /// Allocates a Terrain with a zero generator.
    pub fn new() -> Self {
        Self::with_generator(zero_generator)
    }

    /// Allocates a Terrain with a custom generator, but does not actually generate any chunks.
    pub fn with_generator(generator: Generator) -> Self {
        const NONE_CHUNK: Option<Box<Chunk>> = None;
        const NONE_CHUNK_ROW: [Option<Box<Chunk>>; SIZE_CHUNKS] = [NONE_CHUNK; SIZE_CHUNKS];

        Self {
            chunks: [NONE_CHUNK_ROW; SIZE_CHUNKS],
            updated: ChunkSet::new(),
            mutex: Mutex::new(()),
            generator,
        }
    }

    /// Returns the maximum world radius to not exceed the terrain size, which is effectively a
    /// constant.
    pub fn max_world_radius() -> f32 {
        (SIZE / 2) as f32 * SCALE
    }

    /// Returns a mutable reference to a chunk.
    pub fn mut_chunk(&mut self, chunk_id: ChunkId) -> &mut Chunk {
        let chunk = &mut self.chunks[chunk_id.1 as usize][chunk_id.0 as usize];
        if chunk.is_none() {
            *chunk = Some(Chunk::new(chunk_id, self.generator));
        }
        chunk.as_mut().unwrap()
    }

    /// Gets a reference to a chunk, generating it if necessary.
    #[inline]
    pub fn get_chunk(&self, chunk_id: ChunkId) -> &Chunk {
        unsafe {
            let ptr: &AtomicPtr<Chunk> =
                transmute(&self.chunks[chunk_id.1 as usize][chunk_id.0 as usize]);
            if let Some(chunk) = ptr.load(Ordering::Relaxed).as_ref() {
                return chunk;
            }
            self.get_chunk_slow(ptr, chunk_id)
        }
    }

    #[inline(never)]
    unsafe fn get_chunk_slow(&self, ptr: &AtomicPtr<Chunk>, chunk_id: ChunkId) -> &Chunk {
        let lock = self.mutex.lock().unwrap();
        if let Some(chunk) = ptr.load(Ordering::Acquire).as_ref() {
            return chunk;
        }

        let chunk = Box::into_raw(Chunk::new(chunk_id, self.generator));
        ptr.store(chunk, Ordering::Release);
        drop(lock);
        chunk.as_ref().unwrap()
    }

    /// Resets the updated field to empty.
    pub fn reset_updated(&mut self) {
        self.updated = ChunkSet::new()
    }

    /// Applies a terrain update, overwriting relevant terrain pixels.
    pub fn apply_update(&mut self, update: &TerrainUpdate) {
        for SerializedChunk(chunk_id, chunk_bytes) in update.iter() {
            *self.mut_chunk(*chunk_id) = Chunk::from_bytes(chunk_bytes)
        }
    }

    /// Gets the raw terrain data at a Coord.
    pub fn at(&self, coord: Coord) -> u8 {
        self.get_chunk(ChunkId::from_coord(coord)).at(coord)
    }

    /// Sets the raw terrain data at a Coord.
    fn set(&mut self, coord: Coord, value: u8) {
        let chunk_id = ChunkId::from_coord(coord);
        let chunk = self.mut_chunk(chunk_id);
        chunk.set(coord, value);
        chunk.mark_for_regenerate();
        self.updated.add(chunk_id)
    }

    /// Gets the smoothed Altitude at a position.
    pub fn sample(&self, pos: Vec2) -> Option<Altitude> {
        let pos = pos.mul(1.0 / SCALE);

        let c_pos = pos.ceil();
        let c = Coord::from_scaled_position(c_pos)?;
        let Coord(cx, cy) = c;

        let f_pos = pos.floor();
        let f = Coord::from_scaled_position(f_pos)?;
        let Coord(fx, fy) = f;

        // Sample 2x2 grid
        // 00 10
        // 01 11
        let c00: u8;
        let c10: u8;
        let c01: u8;
        let c11: u8;

        let chunk_id = ChunkId::from_coord(f);
        if chunk_id == ChunkId::from_coord(c) {
            let chunk = self.get_chunk(chunk_id);
            c00 = chunk.at(Coord(fx, fy));
            c10 = chunk.at(Coord(cx, fy));
            c01 = chunk.at(Coord(fx, cy));
            c11 = chunk.at(Coord(cx, cy));
        } else {
            c00 = self.at(Coord(fx, fy));
            c10 = self.at(Coord(cx, fy));
            c01 = self.at(Coord(fx, cy));
            c11 = self.at(Coord(cx, cy));
        }

        // Offset the terrain by 6 units, so that the strata representing sea level is slightly
        // above 0. A typical terrain shader would add a similar amount (on average, via noise) to
        // make islands more interesting (but still smooth).

        let delta = pos.sub(f_pos);
        Some(lookup_altitude(
            lerp(
                lerp(c00 as f32, c10 as f32, delta.x),
                lerp(c01 as f32, c11 as f32, delta.x),
                delta.y,
            ) as u8
                + 6,
        ))
    }

    /// collides_with returns if an entity collides with the terrain given a time step in seconds.
    pub fn collides_with(
        &self,
        mut dim_transform: DimensionTransform,
        threshold: Altitude,
        delta_seconds: f32,
    ) -> bool {
        let normal = dim_transform.transform.direction.to_vec();
        let tangent = Vec2::new(-normal.y, normal.x);

        let sweep = delta_seconds * dim_transform.transform.velocity.to_mps();
        dim_transform.dimensions.x += sweep;
        dim_transform.transform.position += normal * (sweep * 0.5);

        // Not worth doing multiple terrain samples for small, slow moving entities.
        if dim_transform.dimensions.x <= SCALE * 0.2 && dim_transform.dimensions.y <= SCALE * 0.2 {
            if let Some(alt) = self.sample(dim_transform.transform.position) {
                return alt >= threshold;
            }
        }

        // Allow for a small margin of error.
        const GRACE_MARGIN: f32 = 0.9;
        dim_transform.dimensions *= GRACE_MARGIN;

        let dx = (SCALE * 2.0 / 3.0).min(dim_transform.dimensions.x * 0.499);
        let dy = (SCALE * 2.0 / 3.0).min(dim_transform.dimensions.y * 0.499);

        let half_length = (dim_transform.dimensions.x / dx) as i32 / 2;
        let half_width = (dim_transform.dimensions.y / dy) as i32 / 2;

        for l in -(half_length as i32)..=half_length as i32 {
            for w in -(half_width as i32)..=half_width as i32 {
                let l = l as f32 * dx;
                debug_assert!(l > dim_transform.dimensions.x * -0.5);
                debug_assert!(l < dim_transform.dimensions.x * 0.5);
                let w = w as f32 * dy;
                debug_assert!(w > dim_transform.dimensions.y * -0.5);
                debug_assert!(w < dim_transform.dimensions.y * 0.5);
                let pos = dim_transform.transform.position + normal * l + tangent * w;

                if let Some(alt) = self.sample(pos) {
                    if alt >= threshold {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Returns if there is any land in a square, centered at center. Useful for determining whether something can spawn.
    pub fn land_in_square(&self, center: Vec2, side_length: f32) -> bool {
        let lower_left = Coord::saturating_from_position(center - side_length * 0.5);
        let upper_right = Coord::saturating_from_position(center + side_length * 0.5);

        for x in lower_left.0..upper_right.0 {
            for y in lower_left.1..upper_right.1 {
                if self.at(Coord(x, y)) + 6 > 255 / 2 {
                    return true;
                }
            }
        }
        false
    }

    /// Modifies a small radius around a pos by adding or subtracting an amount of land. Returns None
    /// if unsuccessful.
    pub fn modify(&mut self, pos: Vec2, mut amount: f32) -> Option<()> {
        let pos = pos.mul(1.0 / SCALE);

        let c_pos = pos.ceil();
        let Coord(cx, cy) = Coord::from_scaled_position(c_pos)?;

        let f_pos = pos.floor();
        let Coord(fx, fy) = Coord::from_scaled_position(f_pos)?;

        let delta = pos.sub(f_pos);

        // The following code effectively double's the amount, so correct for this.
        amount *= 0.5;

        self.set(
            Coord(fx, fy),
            ((self.at(Coord(fx, fy)) + 0b0011) as f32 + amount * (2.0 - delta.x - delta.y)) as u8,
        );
        self.set(
            Coord(cx, fy),
            ((self.at(Coord(cx, fy)) + 0b0011) as f32 + amount * (1.0 + delta.x - delta.y)) as u8,
        );
        self.set(
            Coord(fx, cy),
            ((self.at(Coord(fx, cy)) + 0b0011) as f32 + amount * (1.0 - delta.x + delta.y)) as u8,
        );
        self.set(
            Coord(cx, cy),
            ((self.at(Coord(cx, cy)) + 0b0011) as f32 + amount * (delta.x + delta.y)) as u8,
        );
        Some(())
    }

    /// Regenerate partially regenerates chunks that are due for regeneration.
    pub fn regenerate_if_applicable(&mut self) {
        let now = Instant::now();

        for (cy, chunks) in self.chunks.iter_mut().enumerate() {
            for (cx, chunk) in chunks.iter_mut().enumerate() {
                if let Some(chunk) = chunk {
                    if let Some(next_regen) = chunk.next_regen {
                        if now > next_regen {
                            let chunk_id = ChunkId(cx as u16, cy as u16);
                            chunk.regenerate(chunk_id, self.generator);
                            self.updated.add(chunk_id);
                        }
                    }
                }
            }
        }
    }
}

/// A single chunk in a Terrain.
pub struct Chunk {
    data: [[u8; CHUNK_SIZE / 2]; CHUNK_SIZE],
    next_regen: Option<Instant>,
}

impl Chunk {
    /// Allocates a zero chunk.
    pub fn zero() -> Self {
        Self {
            data: [[0; CHUNK_SIZE / 2]; CHUNK_SIZE],
            next_regen: None,
        }
    }

    /// Generates a new chunk by invoking generator for each pixel.
    pub fn new(chunk_id: ChunkId, generator: Generator) -> Box<Self> {
        // Ensure array is initialized on the heap, not the stack.
        // See https://github.com/rust-lang/rust/issues/28008#issuecomment-135032399
        let mut chunk = box Self {
            data: [[0; CHUNK_SIZE / 2]; CHUNK_SIZE],
            next_regen: None,
        };

        let coord = chunk_id.as_coord();
        let x_offset = coord.0;
        let y_offset = coord.1;

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                chunk.set(Coord(x, y), generator(x + x_offset, y + y_offset));
            }
        }
        chunk
    }

    /// regenerate brings each pixel of the chunk one unit closer to original height.
    pub fn regenerate(&mut self, chunk_id: ChunkId, generator: Generator) {
        let coord = chunk_id.as_coord();
        let x_offset = coord.0;
        let y_offset = coord.1;

        // Whether the regeneration is incomplete (some pixels are still not equal to original values).
        let mut incomplete = false;

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let coord = Coord(x, y);
                let height = self.at(coord);
                let original_height = generator(x + x_offset, y + y_offset) & 0b11110000;

                let new_height = match original_height.cmp(&height) {
                    std::cmp::Ordering::Less => height - 0b10000,
                    std::cmp::Ordering::Greater => height + 0b10000,
                    std::cmp::Ordering::Equal => continue,
                };

                self.set(coord, new_height);

                if new_height != original_height {
                    incomplete = true;
                }
            }
        }

        self.next_regen = None;

        if incomplete {
            self.mark_for_regenerate();
        }
    }

    /// mark_for_regenerate marks this chunk for regenerating after a standard time delay.
    /// Does nothing if the chunk is already marked as such.
    fn mark_for_regenerate(&mut self) {
        if self.next_regen.is_none() {
            self.next_regen = Some(
                Instant::now()
                    + Duration::from_secs_f32(thread_rng().gen_range(0.75..1.25) * 60.0 * 20.0),
            );
        }
    }

    /// Gets the raw value of one pixel, specified by coord modulo CHUNK_SIZE.
    #[inline]
    pub fn at(&self, coord: Coord) -> u8 {
        let Coord(x, mut y) = coord;
        let sx = (x / 2) % (CHUNK_SIZE / 2);
        y %= CHUNK_SIZE;

        (self.data[y][sx] << ((x & 1) * 4)) & 0b11110000
    }

    /// Sets the raw value of one pixel, specified by coord modulo CHUNK_SIZE.
    pub fn set(&mut self, coord: Coord, value: u8) {
        let Coord(x, mut y) = coord;
        let sx = (x / 2) % (CHUNK_SIZE / 2);
        y %= CHUNK_SIZE;

        let shift = (x & 1) * 4;
        self.data[y][sx] = (self.data[y][sx] & (0b1111 << shift)) | ((value & 0b11110000) >> shift);
    }

    /// to_bytes encodes a chunk as bytes.
    /// It uses run-length encoding of the chunk mapped to a hilbert curve.
    pub fn to_bytes(&self) -> Box<[u8]> {
        let mut compressor = Compressor::new(1024);
        for coord in HILBERT_TO_COORD.iter() {
            compressor.write_byte(self.at((*coord).into()));
        }
        compressor.into_vec().into_boxed_slice()
    }

    /// from_bytes decodes bytes encoded with to_bytes into a chunk.
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut chunk = Self::zero();
        let hilbert = &HILBERT_TO_COORD;
        for (i, b) in Decompressor::new(bytes).enumerate() {
            chunk.set(hilbert[i].into(), b);
        }
        chunk
    }
}

lazy_static! {
    static ref HILBERT_TO_COORD: [(u8, u8); CHUNK_SIZE * CHUNK_SIZE] = {
        let mut lut = [(0u8, 0u8); CHUNK_SIZE * CHUNK_SIZE];
        for (i, v) in lut.iter_mut().enumerate() {
            *v = hilbert::h2xy(i as u16)
        }
        lut
    };
}

/// An efficient set of ChunkIds.
#[derive(Clone, Eq, PartialEq)]
pub struct ChunkSet {
    data: [usize; Self::DATA_SIZE],
}

impl ChunkSet {
    const ROW_SIZE: usize = size_of::<usize>();
    const ROW_SIZE_LOG2: u32 = (Self::ROW_SIZE - 1).count_ones();
    const DATA_SIZE: usize = SIZE_CHUNKS.pow(2) / Self::ROW_SIZE;

    pub fn new() -> Self {
        Self {
            data: [0; Self::DATA_SIZE],
        }
    }

    /// Returns the set of all chunks from within radius of a centor position.
    pub fn new_radius(center: Vec2, radius: f32) -> Self {
        let min_chunk_id = ChunkId::saturating_from(center - radius);
        let max_chunk_id = ChunkId::saturating_from(center + radius);

        let mut result = Self::new();

        for y in min_chunk_id.1..=max_chunk_id.1 {
            for x in min_chunk_id.0..=max_chunk_id.0 {
                let chunk_id = ChunkId(x, y);
                if chunk_id.in_radius(center, radius) {
                    result.add(chunk_id);
                }
            }
        }

        result
    }

    /// Returns true if the set contains a given ChunkId.
    pub fn contains(&self, chunk_id: ChunkId) -> bool {
        self.contains_index(chunk_id.as_index())
    }

    fn contains_index(&self, index: usize) -> bool {
        let row = self.data[index >> Self::ROW_SIZE_LOG2];
        row & 1 << (index % Self::ROW_SIZE) != 0
    }

    /// Inserts a given ChunkId into this set.
    pub fn add(&mut self, chunk_id: ChunkId) {
        self.add_index(chunk_id.as_index());
    }

    fn add_index(&mut self, index: usize) {
        let row = &mut self.data[index >> Self::ROW_SIZE_LOG2];
        *row |= 1 << (index % Self::ROW_SIZE);
    }

    /// Iterates all ChunkIds in the set.
    pub fn into_iter(self) -> impl Iterator<Item = ChunkId> {
        (0..Self::DATA_SIZE * Self::ROW_SIZE)
            .filter(move |i| self.contains_index(*i))
            .map(ChunkId::from_index)
    }

    fn unary_op<F: Fn(usize) -> usize>(&self, op: F) -> Self {
        let mut result = Self::new();
        for (i, a) in self.data.iter().enumerate() {
            result.data[i] = op(*a);
        }
        result
    }

    fn binary_op<F: Fn(usize, usize) -> usize>(&self, other: &Self, op: F) -> Self {
        let mut result = Self::new();
        for (i, (a, b)) in self.data.iter().zip(other.data.iter()).enumerate() {
            result.data[i] = op(*a, *b);
        }
        result
    }

    /// Returns the intersection of two sets.
    pub fn and(&self, other: &Self) -> Self {
        self.binary_op(other, |a, b| a & b)
    }

    /// Returns the union of two sets.
    pub fn or(&self, other: &Self) -> Self {
        self.binary_op(other, |a, b| a | b)
    }

    /// Returns the inverse of this set.
    pub fn not(&self) -> Self {
        self.unary_op(|a| !a)
    }
}

impl Default for ChunkSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ChunkSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}",
            self.clone().into_iter().collect::<Vec<ChunkId>>()
        )
    }
}

struct Compressor {
    buf: Vec<u8>,
}

impl Compressor {
    fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
        }
    }

    fn write_byte(&mut self, b: u8) {
        const MAX_COUNT: u8 = 16;

        let next = b >> 4;

        let n = self.buf.len();
        if n > 0 {
            let last = &mut self.buf[n - 1];

            let tuple = *last;
            let current = tuple >> 4;
            let count_minus_one = tuple % MAX_COUNT;

            // Add to run length if same nibble and count has more space.
            if next == current && count_minus_one < MAX_COUNT - 1 {
                *last += 1;
                return;
            }
        }
        self.buf.push(next << 4);
    }

    fn into_vec(self) -> Vec<u8> {
        self.buf
    }
}

struct Decompressor<'a> {
    buf: &'a [u8],
    off: usize,
    repeat: u8,
}

impl<'a> Decompressor<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self {
            buf,
            off: 0,
            repeat: 0,
        }
    }
}

impl Iterator for Decompressor<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        const MAX_COUNT: u8 = 16;

        if self.off < self.buf.len() {
            let tuple = self.buf[self.off];
            if tuple % MAX_COUNT != self.repeat {
                self.repeat += 1;
            } else {
                self.off += 1;
                self.repeat = 0;
            }

            Some(tuple & 0b11110000)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Rng};

    fn random_generator(_: usize, _: usize) -> u8 {
        thread_rng().gen::<u8>() & 0b11110000
    }

    #[test]
    fn compress() {
        let mut terrain = Terrain::with_generator(random_generator);
        let chunk = terrain.mut_chunk(ChunkId(0, 0));
        let bytes = chunk.to_bytes();
        println!(
            "random chunk: {} compressed vs {} memory",
            bytes.len(),
            size_of::<Chunk>()
        );
        let chunk2 = Chunk::from_bytes(&bytes);
        assert_eq!(chunk.data, chunk2.data);
    }
}
