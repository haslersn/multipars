use criterion::{criterion_group, criterion_main, Criterion};

mod bgv;
mod low_gear;

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = low_gear::criterion_benchmark, bgv::criterion_benchmark
}
criterion_main!(benches);
