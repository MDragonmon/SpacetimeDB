use std::sync::Arc;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use spacetimedb_testing::modules::{compile, with_module};
use tokio::sync::Mutex;

fn criterion_benchmark(c: &mut Criterion) {
    compile("benchmarks");

    with_module("benchmarks", |runtime, module| {
        c.bench_function("empty reducer", |b| {
            b.to_async(runtime).iter(|| async move {
                module.call_reducer("empty", "[]".into()).await.unwrap();
            });
        });
    });

    with_module("benchmarks", |runtime, module| {
        let count = &Arc::new(Mutex::new(0usize));
        c.bench_function("single insert", |b| {
            b.to_async(runtime).iter(|| async move {
                let count_clone = count.clone();
                let mut count_locked = count_clone.lock().await;
                let args = format!(r#"["name {}"]"#, *count_locked);
                module.call_reducer("single_insert", args).await.unwrap();
                *count_locked += 1;
            });
        });
    });

    with_module("benchmarks", |runtime, module| {
        let mut group = c.benchmark_group("multi insert");

        let offset = &Arc::new(Mutex::new(0usize));
        for size in [10, 50, 100, 500, 1000].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
                b.to_async(runtime).iter(|| async move {
                    let offset_clone = offset.clone();
                    let offset_locked = offset_clone.lock().await;
                    let args = format!(r#"[{}, {}]"#, size, *offset_locked);
                    drop(offset_locked);
                    module.call_reducer("multi_insert", args).await.unwrap();
                });
            });
        }
        group.finish();
    });

    with_module("benchmarks", |runtime, module| {
        let mut group = c.benchmark_group("with existing records");
        let mut total = 0;
        let record_id = &Arc::new(Mutex::new(0usize));
        for i in 0..10 {
            let count = 100_000;
            let offset = i * count;
            runtime.block_on(async {
                let args = format!(r#"[{}, {}]"#, count, offset);
                module.call_reducer("multi_insert", args).await.unwrap();
            });

            total += count;

            group.bench_with_input(BenchmarkId::from_parameter(total), &total, |b, _| {
                b.to_async(runtime).iter(|| async {
                    let record_id_clone = record_id.clone();
                    let mut record_id_locked = record_id_clone.lock().await;
                    let args = format!(r#"["name {}"]"#, *record_id_locked);
                    *record_id_locked += 1;
                    drop(record_id_locked);
                    module.call_reducer("single_insert", args).await.unwrap();
                });
            });
        }

        group.finish();
    });

    with_module("benchmarks", |runtime, module| {
        // TODO: when bigger params are merged this should be changed
        // maybe even a group with different sizes
        let size = byte_unit::Byte::from_str("64KB").unwrap().get_bytes() as usize;
        let record_id = &Arc::new(Mutex::new(0usize));
        let name = "0".repeat(size - 4);
        c.bench_function("large input", |b| {
            b.to_async(runtime).iter(|| async {
                let record_id_clone = record_id.clone();
                let mut record_id_locked = record_id_clone.lock().await;
                let args = format!(r#"["{}{}"]"#, &name, record_id_locked);
                *record_id_locked += 1;
                drop(record_id_locked);
                module.call_reducer("single_insert", args).await.unwrap();
            });
        });
    });

    with_module("benchmarks", |runtime, module| {
        // TODO: when bigger params are merged this should be changed
        // maybe even a group with different sizes
        let size = byte_unit::Byte::from_str("64KB").unwrap().get_bytes() as usize;
        let name = &"0".repeat(size - 4);

        let record_id = &Arc::new(Mutex::new(0usize));

        c.bench_function("multiple large arguments", |b| {
            b.to_async(runtime).iter(|| async {
                // TODO: I'm not sue how expensive this might be. I plan to add
                // a helper that deals with preparing the data before hand, but
                // for now this should be OK
                let record_id_clone = record_id.clone();
                let mut record_id_locked = record_id_clone.lock().await;
                let args: String = vec![name; 32]
                    .iter()
                    .map(|s| format!("\"{}{}\"", s, *record_id_locked))
                    .collect::<Vec<String>>()
                    .join(", ");
                *record_id_locked += 1;
                drop(record_id_locked);
                module.call_reducer("a_lot_of_args", args).await.unwrap();
            });
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
