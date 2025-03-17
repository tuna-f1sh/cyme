use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cyme::profiler;
use std::sync::LazyLock;

fn bench_dump() -> profiler::SystemProfile {
    profiler::read_json_dump("./tests/data/cyme_libusb_macos_tree.json").unwrap()
}

static DUMP: LazyLock<profiler::SystemProfile> = LazyLock::new(bench_dump);

pub fn get_node(c: &mut Criterion) {
    let dump = &DUMP;
    c.bench_function("get_device", |b| {
        b.iter(|| {
            let result = dump.get_node("2-2.3.1");
            black_box(result);
        });
    });
    c.bench_function("get_root", |b| {
        b.iter(|| {
            let result = dump.get_node("2-0");
            black_box(result);
        });
    });
}

pub fn get_interface(c: &mut Criterion) {
    let dump = &DUMP;
    c.bench_function("get_interface", |b| {
        b.iter(|| {
            let result = dump.get_interface("20-3.3", 1, 5);
            black_box(result);
        });
    });
}

pub fn get_endpoint(c: &mut Criterion) {
    let dump = &DUMP;
    c.bench_function("get_endpoint", |b| {
        b.iter(|| {
            let result = dump.get_endpoint("20-3.3", 1, 5, 0x85);
            black_box(result);
        });
    });
}

criterion_group!(single_benches, get_node, get_interface, get_endpoint);
criterion_main!(single_benches);
