#![feature(associated_const_equality)]

pub mod bgv;
pub mod bi_channel;
pub mod buffered_preproc;
pub mod connection;
pub mod interface;
pub mod low_gear_dealer;
pub mod low_gear_preproc;
pub mod mac_check_opener;
pub mod oneshot_map;
pub mod util;
pub mod zero_preproc;

pub mod examples {
    use std::error::Error;
    use std::time::Instant;

    use crate::connection::Connection;
    use crate::interface::BatchedPreprocessor;
    use crate::low_gear_preproc::{self, LowGearPreprocessor, PreprocessorParameters};
    use crate::util::resolve_host;

    pub async fn low_gear<PreprocParams, const PID: usize>(
        local: &str,
        remote: &str,
        num_threads: usize,
        num_batches: usize,
    ) -> Result<(), Box<dyn Error>>
    where
        PreprocParams: PreprocessorParameters,
    {
        let local_addr = local.parse()?;
        let remote_addr = resolve_host(remote)?;

        let mut conn = Connection::new(local_addr, remote_addr).await?;

        tokio::task::spawn_blocking(move || {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(num_threads)
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut conns = Vec::new();
                    for _ in 0..num_batches {
                        conns.push(conn.fork());
                    }
                    let preprocs: Vec<_> =
                        futures_util::future::join_all(conns.into_iter().map(|mut conn| {
                            tokio::task::spawn(async move {
                                LowGearPreprocessor::<PreprocParams, PID>::new(&mut conn)
                                    .await
                                    .unwrap()
                            })
                        }))
                        .await;

                    let now = Instant::now();

                    let preprocs: Vec<_> = futures_util::future::join_all(
                        preprocs.into_iter().map(Result::unwrap).map(|mut preproc| {
                            tokio::task::spawn(async move {
                                preproc.get_beaver_triples().await;
                                preproc
                            })
                        }),
                    )
                    .await;

                    let elapsed_time = now.elapsed();
                    let num_triples = low_gear_preproc::batch_size::<PreprocParams>() * num_batches;
                    println!(
                        "{} triples/s (produced {} triples in {} ms)",
                        num_triples as f64 * 1_000_000_000f64 / elapsed_time.as_nanos() as f64,
                        num_triples,
                        elapsed_time.as_millis()
                    );

                    for preproc in preprocs.into_iter() {
                        preproc.unwrap().finish().await;
                    }
                })
        })
        .await?;
        Ok(())
    }
}
