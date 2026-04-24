use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use mainnet_observer_backend::rest::Block;
use mainnet_observer_backend::stats::Stats;
use std::fs;
use std::path::Path;

fn deserialize_block(json: &str) -> Block {
    serde_json::from_str(json).expect("failed to deserialize block fixture")
}

fn bench_stats_from_block(c: &mut Criterion) {
    let fixtures_dir = Path::new("testdata/");
    if !fixtures_dir.exists() {
        eprintln!("No fixtures found.");
        return;
    }

    let mut fixtures: Vec<(String, String)> = Vec::new();
    for entry in fs::read_dir(fixtures_dir).expect("read fixtures dir") {
        let entry = entry.expect("read dir entry");
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let json = fs::read_to_string(&path).expect("read fixture");
            fixtures.push((name, json));
        }
    }

    if fixtures.is_empty() {
        eprintln!("No .json fixtures in testdata/.");
        return;
    }

    fixtures.sort_by(|a, b| a.0.cmp(&b.0));

    // Benchmark: full Stats::from_block
    {
        let mut group = c.benchmark_group("Stats::from_block");
        for (name, json) in &fixtures {
            group.bench_with_input(BenchmarkId::new("full", name), json, |b, json| {
                b.iter_batched(
                    || deserialize_block(json),
                    |block| Stats::from_block(block).unwrap(),
                    criterion::BatchSize::SmallInput,
                );
            });
        }
        group.finish();
    }

    // Benchmark: JSON deserialization only
    {
        let mut group = c.benchmark_group("block_deserialization");
        for (name, json) in &fixtures {
            group.bench_with_input(BenchmarkId::new("json", name), json, |b, json| {
                b.iter(|| deserialize_block(json));
            });
        }
        group.finish();
    }
}

criterion_group!(benches, bench_stats_from_block);
criterion_main!(benches);
