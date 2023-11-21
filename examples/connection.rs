use std::error::Error;

use async_bincode::tokio::{AsyncBincodeReader, AsyncBincodeWriter};
use futures_util::{SinkExt, StreamExt};
use multipars::connection::Connection;
use tokio::task::JoinError;

#[tokio::main]
async fn main() -> Result<(), JoinError> {
    const P0_ADDR: &str = "[::1]:50051";
    const P1_ADDR: &str = "[::1]:50052";

    env_logger::init();
    tokio::try_join!(
        tokio::task::spawn(async move {
            run_party(P0_ADDR, P1_ADDR).await.unwrap();
        }),
        tokio::task::spawn(async move {
            run_party(P1_ADDR, P0_ADDR).await.unwrap();
        }),
    )
    .map(drop)
}

async fn run_party(local: &str, remote: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let local_addr = local.parse().unwrap();
    let remote_addr = remote.parse().unwrap();

    let mut conn1 = Connection::new(local_addr, remote_addr).await?;
    let mut conn2 = conn1.fork();
    let mut conn3 = conn1.fork();
    let mut conn4 = conn1.fork();

    tokio::try_join!(
        open_bi_and_exchange_i32(local, &mut conn1, 1),
        open_bi_and_exchange_i32(local, &mut conn2, 2),
        open_bi_and_exchange_i32(local, &mut conn3, 3),
        open_bi_and_exchange_i32(local, &mut conn4, 4),
    )?;

    Ok(())
}

async fn open_bi_and_exchange_i32(
    listen_addr: &str,
    conn: &mut Connection,
    payload: i32,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (mut tx, mut rx) = conn.open_bi().await?;
    AsyncBincodeWriter::from(&mut tx)
        .for_async()
        .send(payload)
        .await?;
    let received: i32 = AsyncBincodeReader::from(&mut rx).next().await.unwrap()?;
    println!(
        "{}: Expected payload {}, received payload {}",
        listen_addr, payload, received
    );
    let _ = tx.finish().await;
    Ok(())
}
