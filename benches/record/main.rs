mod any_map;
mod enum_map;

use criterion::{criterion_group, criterion_main, Criterion};

fn empty(c: &mut Criterion) {
    c.bench_function("empty", |b| {
        b.iter(|| {
            std::hint::black_box(());
        })
    });
}

fn any_map_serialize(c: &mut Criterion) {
    let mut record = any_map::Record::new();
    record.insert("key", "value".to_owned());
    let s = serde_json::to_string(&record).unwrap();
    println!("{s}");
}

criterion_group!(record, empty, any_map_serialize,);

criterion_main!(record);
