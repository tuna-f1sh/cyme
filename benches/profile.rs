use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cyme::profiler;

pub fn profile(c: &mut Criterion) {
    c.bench_function("get_system_profile", |b| {
        b.iter(|| {
            black_box(profiler::get_spusb_with_extra().unwrap());
        });
    });
    #[cfg(target_os = "macos")]
    c.bench_function("get_system_profile_sp", |b| {
        b.iter(|| {
            black_box(profiler::macos::get_spusb().unwrap());
        });
    });
}

criterion_group!(single_benches, profile);
criterion_main!(single_benches);
