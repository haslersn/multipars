use std::fmt::Debug;
use std::future::Future;
use std::time::{Duration, Instant};

use criterion::{Bencher, Criterion};
use multipars::low_gear_preproc::params::ToyPreprocK32S32;
use multipars::low_gear_preproc::PreprocessorParameters;
use multipars::{examples, low_gear_preproc};
use tokio::runtime::Runtime;

const P0_ADDR: &str = "[::1]:50051";
const P1_ADDR: &str = "[::1]:50052";

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("low_gear");

    group.bench_function("toy_k32_s32", |b| bench_low_gear::<ToyPreprocK32S32>(b));
}

async fn time<V, E: Debug>(fut: impl Future<Output = Result<V, E>>, denominator: u32) -> Duration {
    let start = Instant::now();
    fut.await.unwrap();
    start.elapsed() / denominator
}

fn bench_low_gear<PreprocParams>(b: &mut Bencher)
where
    PreprocParams: PreprocessorParameters,
{
    b.to_async(Runtime::new().unwrap())
        .iter_custom(|num_iterations| {
            time(
                async move {
                    tokio::try_join!(
                        tokio::task::spawn(async move {
                            examples::low_gear::<PreprocParams, 0>(
                                P0_ADDR,
                                P1_ADDR,
                                num_iterations as usize, // TODO: Maybe too many parallel tasks
                                num_iterations as usize, // TODO: Maybe too many parallel tasks
                            )
                            .await
                            .unwrap();
                        }),
                        tokio::task::spawn(async move {
                            examples::low_gear::<PreprocParams, 1>(
                                P1_ADDR,
                                P0_ADDR,
                                num_iterations as usize, // TODO: Maybe too many parallel tasks
                                num_iterations as usize, // TODO: Maybe too many parallel tasks
                            )
                            .await
                            .unwrap();
                        }),
                    )
                    .map(drop)
                },
                low_gear_preproc::batch_size::<PreprocParams>() as u32,
            )
        })
}
