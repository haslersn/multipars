use clap::Parser;
use multipars::{
    examples,
    low_gear_preproc::{
        params::{PreprocK128S64, PreprocK32S32, PreprocK64S64, ToyPreprocK32S32},
        PreprocessorParameters,
    },
};

#[derive(Clone, Debug, Parser)]
struct Args {
    #[arg(long, default_value_t = String::from("[::1]:50051"))]
    p0_addr: String,

    #[arg(long, default_value_t = String::from("[::1]:50052"))]
    p1_addr: String,

    #[arg(long, value_enum, default_value_t = Player::Both)]
    player: Player,

    #[arg(long, default_value_t = 1)]
    batches: usize,

    #[arg(long, default_value_t = 1)]
    threads: usize,

    #[arg(short, default_value_t = 32)]
    k: usize,

    #[arg(short, default_value_t = 32)]
    s: usize,

    #[arg(long, default_value_t = false)]
    toy: bool,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum Player {
    Zero,
    One,
    Both,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Args::parse();
    match (args.toy, args.k, args.s) {
        (true, 32, 32) => run::<ToyPreprocK32S32>(args).await,
        (false, 32, 32) => run::<PreprocK32S32>(args).await,
        (false, 64, 64) => run::<PreprocK64S64>(args).await,
        (false, 128, 64) => run::<PreprocK128S64>(args).await,
        _ => {
            panic!("unsupported combination");
        }
    }
}

async fn run<PreprocParams>(args: Args)
where
    PreprocParams: PreprocessorParameters,
{
    let task_p0 = run_player::<PreprocParams, 0>(
        args.p0_addr.clone(),
        args.p1_addr.clone(),
        args.threads,
        args.batches,
    );
    let task_p1 = run_player::<PreprocParams, 1>(
        args.p1_addr.clone(),
        args.p0_addr.clone(),
        args.threads,
        args.batches,
    );

    match args.player {
        Player::Zero => task_p0.await,
        Player::One => task_p1.await,
        Player::Both => {
            tokio::try_join!(tokio::task::spawn(task_p0), tokio::task::spawn(task_p1)).unwrap();
        }
    }
}

async fn run_player<PreprocParams, const PID: usize>(
    local_addr: String,
    remote_addr: String,
    num_threads: usize,
    num_batches: usize,
) where
    PreprocParams: PreprocessorParameters,
{
    examples::low_gear::<PreprocParams, PID>(&local_addr, &remote_addr, num_threads, num_batches)
        .await
        .unwrap();
}
