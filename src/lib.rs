#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
use std::io::{Cursor, ErrorKind, Read};
#[cfg_attr(not(feature = "std"), allow(unused))]
use core::hash::Hasher;

/// Calculate the 32-bit hash of the provided slice `slice` using the initial seed `seed`.
///
/// This should be preferred over [`hash_streaming()`] for input that's already been loaded to
/// memory and has a known length, as it produces tighter assembly.
pub fn hash(slice: &[u8], seed: u64) -> u32 {
    marvin32_hash(slice, seed)
}

#[cfg(feature = "std")]
/// Calculate the 32-bit hash of the provided `source` using the initial seed `seed`.
///
/// Only returns an error in case reading from `source` failed. Prefer to use `[hash()]` if
/// the input has already been loaded into memory as it is faster.
pub fn hash_streaming<R: Read>(source: &mut R, seed: u64) -> std::io::Result<u32> {
    marvin32_hash_streaming(source, seed)
}

#[cfg(feature = "std")]
/// An `[std::hash::Hasher]` implementation using the marvin32 hash algorithm.
pub struct Marvin32 {
    state: Marvin32State,
    buffer: Cursor<[u8; 4]>,
}

#[cfg_attr(feature = "std", derive(Clone))]
struct Marvin32State {
    lo: u32,
    hi: u32,
}

#[inline(always)]
fn marvin32_mix(st: &mut Marvin32State, v: u32) {
    // eprintln!("state: {:#x}; mixing in {v:#x}", ((st.hi as u64) << 32 | st.lo as u64));
    st.lo = st.lo.wrapping_add(v);
    st.hi ^= st.lo;
    st.lo = st.lo.rotate_left(20).wrapping_add(st.hi);
    st.hi = st.hi.rotate_left(9) ^ st.lo;
    st.lo = st.lo.rotate_left(27).wrapping_add(st.hi);
    st.hi = st.hi.rotate_left(19);
}

fn marvin32_hash(ptr: &[u8], seed: u64) -> u32 {
    let mut state = Marvin32State {
        lo: seed as u32,
        hi: (seed >> 32) as u32,
    };

    let mut chunks = ptr.chunks_exact(4);
    while let Some(chunk) = chunks.next() {
        let value = u32::from_le_bytes(chunk.try_into().unwrap());
        marvin32_mix(&mut state, value);
    }
    let final_value = chunks
        .remainder()
        .iter()
        .rev()
        .fold(0x80, |state, byte| (state << 8) | *byte as u32);

    marvin32_mix(&mut state, final_value);
    marvin32_mix(&mut state, 0);

    state.lo ^ state.hi
}

#[cfg(feature = "std")]
fn marvin32_hash_streaming<R: Read>(source: &mut R, seed: u64) -> std::io::Result<u32> {
    let mut state = Marvin32State {
        lo: seed as u32,
        hi: (seed >> 32) as u32,
    };

    let mut buffer = [0u8; 4];
    let final_value = loop {
        match read_chunked(&mut *source, &mut buffer)? {
            n if n > 4 => unsafe { core::hint::unreachable_unchecked() },
            4 => {
                let value = u32::from_le_bytes(buffer);
                marvin32_mix(&mut state, value);
            }
            n => {
                break buffer[0..n]
                    .iter()
                    .rev()
                    .fold(0x80, |state, byte| (state << 8) | *byte as u32);
            }
        }
    };

    marvin32_mix(&mut state, final_value);
    marvin32_mix(&mut state, 0);

    Ok(state.lo ^ state.hi)
}

#[cfg(feature = "std")]
fn read_chunked<R: Read, const C: usize>(src: &mut R, dst: &mut [u8; C]) -> std::io::Result<usize> {
    let mut offset = 0;
    loop {
        if offset >= dst.len() {
            unsafe { core::hint::unreachable_unchecked(); }
        }
        match src.read(&mut dst[offset..]) {
            Ok(0) => return Ok(offset),
            Ok(n) => {
                offset += n;
                if offset == C {
                    return Ok(C);
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::Interrupted {
                    continue;
                }
                return Err(e);
            }
        }
    }
}

#[cfg(feature = "std")]
impl Marvin32 {
    pub fn new(seed: u64) -> Marvin32 {
        Self {
            state: Marvin32State {
                lo: seed as u32,
                hi: (seed >> 32) as u32,
            },
            buffer: Cursor::new([0u8; 4]),
        }
    }
}

#[cfg(feature = "std")]
impl Hasher for Marvin32 {
    fn write(&mut self, mut slice: &[u8]) {
        use std::io::Write;

        // Assert we never start with a full buffer
        debug_assert!(self.buffer.position() != 4);
        // We need to consume our buffer first (by reaching 4 bytes)
        let bytes_to_steal = 4 - self.buffer.position() as usize;
        if bytes_to_steal < 4 {
            // Safe to unwrap since writes to an array-backed Cursor never fail
            // Using write() instead of write_all() because it's faster and there's
            // no need to use write_all() with an array-backed Cursor.
            #[cfg(debug_assertions)]
            let bytes_written = self.buffer.write(&slice[..bytes_to_steal]).unwrap();
            #[cfg(not(debug_assertions))]
            let bytes_written = unsafe { self.buffer.write(&slice[..bytes_to_steal]).unwrap_unchecked() };
            debug_assert_eq!(bytes_written, slice.len().min(bytes_to_steal));
            if bytes_written == bytes_to_steal {
                // We have a full buffer now
                let value = u32::from_le_bytes(self.buffer.get_ref().as_slice().try_into().unwrap());
                self.buffer.set_position(0);
                marvin32_mix(&mut self.state, value);
            }
            slice = &slice[bytes_written..];
        }
        let mut chunks = slice.chunks_exact(4);
        while let Some(chunk) = chunks.next() {
            let value = u32::from_le_bytes(chunk.try_into().unwrap());
            marvin32_mix(&mut self.state, value);
        }
        // Handle any leftover bytes
        let bytes_written = self.buffer.write(chunks.remainder());
        if cfg!(debug_assertions) {
            let bytes_written = bytes_written.unwrap();
            debug_assert_eq!(bytes_written, chunks.remainder().len());
            debug_assert!(bytes_written < 4);
        }
    }

    fn finish(&self) -> u64 {
        let final_value = self.buffer.get_ref()[..self.buffer.position() as usize]
            .iter()
            .rev()
            .fold(0x80, |state, byte| (state << 8) | *byte as u32);
        let mut state = self.state.clone();
        marvin32_mix(&mut state, final_value);
        marvin32_mix(&mut state, 0);
        (state.lo ^ state.hi).into()
    }
}

#[test]
fn unit_test() {
    const TEST: &'static [u8] = b"A\0b\0c\0d\0e\0f\0g\0"; // "Abcdefg" in UTF-16-LE
    assert_eq!(TEST.len(), 14);
    let hash = marvin32_hash(TEST, 0x5D70D359C498B3F8);
    assert_eq!(hash, 0xba627c81, "mismatch in hash");
}

#[test]
#[cfg(feature = "std")]
fn unit_test_streaming() -> std::io::Result<()> {
    const TEST: &'static [u8] = b"A\0b\0c\0d\0e\0f\0g\0"; // "Abcdefg" in UTF-16-LE
    let mut cursor = std::io::Cursor::new(TEST);
    let hash = marvin32_hash_streaming(&mut cursor, 0x5D70D359C498B3F8)?;
    assert_eq!(hash, 0xba627c81, "mismatch in hash");
    Ok(())
}

#[test]
#[cfg(feature = "std")]
fn unit_test_hasher() -> std::io::Result<()> {
    const TEST: &'static [u8] = b"A\0b\0c\0d\0e\0f\0g\0"; // "Abcdefg" in UTF-16-LE
    let mut hash = Marvin32::new(0x5D70D359C498B3F8);
    hash.write(TEST);
    assert_eq!(hash.finish(), 0xba627c81, "mismatch in hash");
    Ok(())
}
