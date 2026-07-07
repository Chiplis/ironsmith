use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use ironsmith::bench_support::{EffectMix, battlefield_scale};
use std::hint::black_box;

fn scale_benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("scale");

    for n in [100usize, 1_000] {
        group.bench_function(format!("chars_cold/n{n}/anthems4"), |b| {
            b.iter_batched(
                || battlefield_scale(n, EffectMix::Anthems(4)),
                |scenario| {
                    for id in &scenario.battlefield {
                        black_box(scenario.game.calculated_characteristics(*id));
                    }
                    black_box(scenario.game.work_counters())
                },
                BatchSize::LargeInput,
            );
        });

        group.bench_function(format!("checkpoint_clone/n{n}"), |b| {
            b.iter_batched(
                || battlefield_scale(n, EffectMix::None).game,
                |game| black_box(game.clone()),
                BatchSize::LargeInput,
            );
        });
    }

    group.bench_function("chars_cold/n500/complex72", |b| {
        b.iter_batched(
            || battlefield_scale(500, EffectMix::ComplexLayerCake(72)),
            |scenario| {
                for id in &scenario.battlefield {
                    black_box(scenario.game.calculated_characteristics(*id));
                }
                black_box(scenario.game.work_counters())
            },
            BatchSize::LargeInput,
        );
    });

    group.bench_function("chars_warm/n500/complex72", |b| {
        let scenario = battlefield_scale(500, EffectMix::ComplexLayerCake(72));
        for id in &scenario.battlefield {
            black_box(scenario.game.calculated_characteristics(*id));
        }
        b.iter(|| {
            for id in &scenario.battlefield {
                black_box(scenario.game.calculated_characteristics(*id));
            }
            black_box(scenario.game.work_counters())
        });
    });

    group.finish();
}

criterion_group!(benches, scale_benches);
criterion_main!(benches);
