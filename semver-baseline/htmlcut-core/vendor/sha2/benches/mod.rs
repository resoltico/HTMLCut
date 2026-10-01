//! Stable benchmark adapter for the upstream SHA-256/SHA-512 update workloads.

use std::hint::black_box;
use std::time::Instant;

use sha2::{Digest, Sha256, Sha512};

fn update<D: Digest + Default>(name: &str, size: usize) {
    let input = vec![0_u8; size];
    let iterations = (16 * 1024 * 1024) / size;
    let mut digest = D::default();
    let started = Instant::now();
    for _ in 0..iterations {
        digest.update(black_box(&input));
    }
    black_box(digest.finalize());
    println!(
        "{name}_{size}: {} bytes in {:?}",
        iterations * size,
        started.elapsed()
    );
}

fn main() {
    for size in [10, 100, 1000, 10000] {
        update::<Sha256>("sha256", size);
        update::<Sha512>("sha512", size);
    }
}
