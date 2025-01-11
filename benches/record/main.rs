use criterion::{criterion_group, criterion_main, Criterion};

fn empty(c: &mut Criterion) {
    c.bench_function("empty", |b| {
        b.iter(|| {
            std::hint::black_box(());
        })
    });
}

criterion_group!(record, empty);

criterion_main!(record);
