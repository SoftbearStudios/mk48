use fxhash::FxHasher32;
use std::hash::Hasher;

pub fn hash_f32<H: Hasher>(f: f32, state: &mut H) {
    state.write_u32(if f == 0.0 || f.is_nan() {
        debug_assert!(!f.is_nan(), "hash_float(NaN)");
        0
    } else {
        f.to_bits()
    });
}

pub fn hash_f32s<H: Hasher, const N: usize>(floats: impl AsRef<[f32; N]>, state: &mut H) {
    for float in floats.as_ref() {
        hash_f32(*float, state);
    }
}

/// A hasher that converts usize to u32 and all integers to little endian bytes for compatibility.
#[derive(Default)]
pub struct CompatHasher {
    inner: FxHasher32,
}

macro_rules! impl_write {
    ($t:ty, $f:ident) => {
        #[inline]
        fn $f(&mut self, i: $t) {
            self.write(&i.to_le_bytes())
        }
    };
}

impl Hasher for CompatHasher {
    fn finish(&self) -> u64 {
        self.inner.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.inner.write(bytes)
    }

    fn write_usize(&mut self, i: usize) {
        self.write_u32(i as u32); // Shouldn't be using any more than 32 bits of usize.
    }

    impl_write!(u8, write_u8);
    impl_write!(u16, write_u16);
    impl_write!(u32, write_u32);
    impl_write!(u64, write_u64);
    impl_write!(u128, write_u128);
}

#[cfg(test)]
mod tests {
    use crate::hash::CompatHasher;
    use std::hash::Hasher;

    #[test]
    fn compat_hasher() {
        const N: u32 = 0x01000193;

        let mut a = CompatHasher::default();
        let mut b = CompatHasher::default();
        a.write_usize(N as usize);
        b.write_u32(N as u32);
        assert_eq!(a.finish(), b.finish());
    }
}
