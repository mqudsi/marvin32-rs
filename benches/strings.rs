// use divan::counter::CharsCount
use divan::counter::BytesCount;

struct Marvin32;
struct SipHash32;

fn main() {
    divan::main();
}

#[divan::bench(
    types = [
        Marvin32,
        SipHash32,
    ],
    consts = [3,6,10,20,40,60,100,250,500,1000],
)]
fn strings<H: Hasher, const LEN: usize>(bencher: divan::Bencher) {
    bencher
        // .counter({
        //     // Constant across inputs.
        //     CharsCount::new(LEN)
        // })
        .with_inputs(|| -> String {
            (0..LEN).map(|_| fastrand::char(..)).collect()
        })
        .input_counter(|s: &String| {
            // Changes based on input.
            BytesCount::of_str(s)
        })
        .bench_refs(|s: &mut String| {
            marvin::hash(s.as_bytes(), 0);
        });
}

trait Hasher {
    // Same signature as marvin32::hash()
    fn hash(slice: &[u8], seed: u64) -> u32;
}

impl Hasher for SipHash32 {
    fn hash(slice: &[u8], seed: u64) -> u32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        seed.hash(&mut hasher);
        slice.hash(&mut hasher);
        // Truncate output to only 32-bits
        hasher.finish() as u32
    }
}

impl Hasher for Marvin32 {
    fn hash(slice: &[u8], seed: u64) -> u32 {
        marvin::hash(slice, seed)
    }
}
