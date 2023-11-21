use criterion::{black_box, AsyncBencher, Bencher, Criterion};
use crypto_bigint::Random;
use multipars::bgv::{
    decrypt, encrypt,
    params::{ToyBgv, ToyCipher, ToyPlain},
    poly::{
        crt::{CrtPoly, CrtPolyParameters},
        power::PowerPoly,
        CrtContext, Diagonal, PolyParameters,
    },
    residue::GenericResidue,
    sample_centered_binomial, PublicKey, SecretKey,
};
use tokio::runtime::Runtime;

pub fn criterion_benchmark(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("bgv");

    group.bench_function("serde_serialize_ciphertext", |b| {
        Runtime::new().unwrap().block_on(async {
            let mut rng = rand::thread_rng();
            let ctx = CrtContext::gen().await;
            let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
            let pk = PublicKey::gen(&ctx, &sk).await;
            let plaintext = PowerPoly::random(&mut rng);
            let ciphertext = encrypt(&ctx, &pk, &plaintext).await;
            b.iter(|| bincode::serialize(black_box(&ciphertext)))
        })
    });

    group.bench_function("serde_deserialize_ciphertext", |b| {
        Runtime::new().unwrap().block_on(async {
            let mut rng = rand::thread_rng();
            let ctx = CrtContext::gen().await;
            let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
            let pk = PublicKey::gen(&ctx, &sk).await;
            let plaintext = PowerPoly::random(&mut rng);
            let ciphertext = encrypt(&ctx, &pk, &plaintext).await;
            b.iter(|| bincode::serialize(black_box(&ciphertext)))
        })
    });

    group.bench_function("sample_centered_binomial", |b| {
        b.iter(|| {
            sample_centered_binomial::<ToyCipher>(20);
        })
    });

    // TODO: first Residue must implement Neg
    //
    // group.bench_function("ciphertext_residue_neg", residue_neg::<ToyCipher>);
    //
    // group.bench_function("plaintext_residue_neg", residue_neg::<ToyPlain>);

    group.bench_function(
        "ciphertext_residue_add",
        residue_add::<<ToyCipher as PolyParameters>::Residue>,
    );

    group.bench_function(
        "plaintext_residue_add",
        residue_add::<<ToyPlain as PolyParameters>::Residue>,
    );

    group.bench_function(
        "ciphertext_residue_sub",
        residue_sub::<<ToyCipher as PolyParameters>::Residue>,
    );

    group.bench_function(
        "plaintext_residue_sub",
        residue_sub::<<ToyPlain as PolyParameters>::Residue>,
    );

    group.bench_function(
        "ciphertext_residue_mul",
        residue_mul::<<ToyCipher as PolyParameters>::Residue>,
    );

    group.bench_function(
        "plaintext_residue_mul",
        residue_mul::<<ToyPlain as PolyParameters>::Residue>,
    );

    group.bench_function("ciphertext_power_poly_add", power_poly_add::<ToyCipher>);

    group.bench_function("plaintext_power_poly_add", power_poly_add::<ToyPlain>);

    group.bench_function("ciphertext_power_poly_sub", power_poly_sub::<ToyCipher>);

    group.bench_function("plaintext_power_poly_sub", power_poly_sub::<ToyPlain>);

    group.bench_function(
        "ciphertext_power_poly_mul_const",
        power_poly_mul_const::<ToyCipher>,
    );

    group.bench_function(
        "plaintext_power_poly_mul_const",
        power_poly_mul_const::<ToyPlain>,
    );

    group.bench_function("ciphertext_crt_poly_add", crt_poly_add::<ToyCipher>);

    group.bench_function("plaintext_crt_poly_add", crt_poly_add::<ToyPlain>);

    group.bench_function(
        "ciphertext_crt_poly_add_const",
        crt_poly_add_const::<ToyCipher>,
    );

    group.bench_function(
        "plaintext_crt_poly_add_const",
        crt_poly_add_const::<ToyPlain>,
    );

    group.bench_function("ciphertext_crt_poly_sub", crt_poly_sub::<ToyCipher>);

    group.bench_function("plaintext_crt_poly_sub", crt_poly_sub::<ToyPlain>);

    group.bench_function(
        "ciphertext_crt_poly_sub_const",
        crt_poly_sub_const::<ToyCipher>,
    );

    group.bench_function(
        "plaintext_crt_poly_sub_const",
        crt_poly_sub_const::<ToyPlain>,
    );

    group.bench_function("ciphertext_crt_poly_mul", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(crt_poly_mul::<ToyCipher>(b))
    });

    group.bench_function("plaintext_crt_poly_mul", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(crt_poly_mul::<ToyPlain>(b))
    });

    group.bench_function(
        "ciphertext_crt_poly_mul_const",
        crt_poly_mul_const::<ToyCipher>,
    );

    group.bench_function(
        "plaintext_crt_poly_mul_const",
        crt_poly_mul_const::<ToyPlain>,
    );

    group.bench_function("ciphertext_power_from_crt", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(power_from_crt::<ToyCipher>(&mut b.to_async(&runtime)));
    });

    group.bench_function("plaintext_power_from_crt", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(power_from_crt::<ToyPlain>(&mut b.to_async(&runtime)));
    });

    group.bench_function("ciphertext_crt_from_power", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(crt_from_power::<ToyCipher>(&mut b.to_async(&runtime)));
    });

    group.bench_function("plaintext_crt_from_power", |b| {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(crt_from_power::<ToyPlain>(&mut b.to_async(&runtime)));
    });

    group.bench_function("encrypt", |b| {
        let runtime = Runtime::new().unwrap();
        let mut b = b.to_async(&runtime);
        runtime.block_on(async {
            let mut rng = rand::thread_rng();
            let ctx = CrtContext::gen().await;
            let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
            let pk = PublicKey::gen(&ctx, &sk).await;
            let plaintext = PowerPoly::random(&mut rng);
            b.iter(|| encrypt(&ctx, &pk, black_box(&plaintext)))
        });
    });

    group.bench_function("decrypt", |b| {
        let runtime = Runtime::new().unwrap();
        let mut b = b.to_async(&runtime);
        runtime.block_on(async {
            let mut rng = rand::thread_rng();
            let ctx = CrtContext::gen().await;
            let sk = SecretKey::<ToyBgv>::gen(&ctx).await;
            let pk = PublicKey::gen(&ctx, &sk).await;
            let plaintext = PowerPoly::random(&mut rng);
            let ciphertext = encrypt(&ctx, &pk, &plaintext).await;
            b.iter(|| decrypt(&ctx, &sk, black_box(&ciphertext)))
        })
    });
}

// TODO: first Residue must implement Neg
// fn residue_neg<Residue>(b: &mut Bencher)
// where
//     Residue: GenericResidue,
// {
//     let mut rng = rand::thread_rng();
//     let lhs = Residue::random(&mut rng);
//     b.iter(|| -black_box(lhs));
// }

fn residue_add<Residue>(b: &mut Bencher)
where
    Residue: GenericResidue,
{
    let mut rng = rand::thread_rng();
    let lhs = Residue::random(&mut rng);
    let rhs = Residue::random(&mut rng);
    b.iter(|| black_box(lhs) + black_box(rhs));
}

fn residue_sub<Residue>(b: &mut Bencher)
where
    Residue: GenericResidue,
{
    let mut rng = rand::thread_rng();
    let lhs = Residue::random(&mut rng);
    let rhs = Residue::random(&mut rng);
    b.iter(|| black_box(lhs) - black_box(rhs));
}

fn residue_mul<Residue>(b: &mut Bencher)
where
    Residue: GenericResidue,
{
    let mut rng = rand::thread_rng();
    let lhs = Residue::random(&mut rng);
    let rhs = Residue::random(&mut rng);
    b.iter(|| black_box(lhs) * black_box(rhs));
}

fn power_poly_add<P>(b: &mut Bencher)
where
    P: PolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = PowerPoly::<P>::random(&mut rng);
    let rhs = PowerPoly::random(&mut rng);
    b.iter(|| {
        lhs += black_box(&rhs);
    });
}

fn power_poly_sub<P>(b: &mut Bencher)
where
    P: PolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = PowerPoly::<P>::random(&mut rng);
    let rhs = PowerPoly::random(&mut rng);
    b.iter(|| {
        lhs -= black_box(&rhs);
    });
}

fn power_poly_mul_const<P>(b: &mut Bencher)
where
    P: PolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = PowerPoly::<P>::random(&mut rng);
    let rhs = Diagonal(<P as PolyParameters>::Residue::random(&mut rng));
    b.iter(|| {
        lhs *= black_box(rhs);
    });
}

fn crt_poly_add<P>(b: &mut Bencher)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = CrtPoly::<P>::random(&mut rng);
    let rhs = CrtPoly::random(&mut rng);
    b.iter(|| {
        lhs += black_box(&rhs);
    });
}

fn crt_poly_add_const<P>(b: &mut Bencher)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = CrtPoly::<P>::random(&mut rng);
    let rhs = Diagonal(<P as PolyParameters>::Residue::random(&mut rng));
    b.iter(|| {
        lhs += black_box(rhs);
    })
}
fn crt_poly_sub<P>(b: &mut Bencher)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = CrtPoly::<P>::random(&mut rng);
    let rhs = CrtPoly::random(&mut rng);
    b.iter(|| {
        lhs -= black_box(&rhs);
    });
}

fn crt_poly_sub_const<P>(b: &mut Bencher)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = CrtPoly::<P>::random(&mut rng);
    let rhs = Diagonal(<P as PolyParameters>::Residue::random(&mut rng));
    b.iter(|| {
        lhs -= black_box(rhs);
    });
}

async fn crt_poly_mul<P>(b: &mut Bencher<'_>)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = CrtPoly::<P>::random(&mut rng);
    let rhs = CrtPoly::random(&mut rng);
    let ctx = CrtContext::gen().await;
    b.iter(|| {
        lhs *= black_box((&rhs, &ctx));
    });
}

fn crt_poly_mul_const<P>(b: &mut Bencher)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let mut lhs = CrtPoly::<P>::random(&mut rng);
    let rhs = Diagonal(<P as PolyParameters>::Residue::random(&mut rng));
    b.iter(|| {
        lhs *= black_box(rhs);
    });
}

async fn power_from_crt<P>(b: &mut AsyncBencher<'_, '_, &Runtime>)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let ctx = CrtContext::gen().await;
    let crt = CrtPoly::<P>::random(&mut rng);
    b.iter(|| async {
        let mut power = PowerPoly::new();
        power.clone_from_crt(&ctx, black_box(&crt)).await;
    })
}

async fn crt_from_power<P>(b: &mut AsyncBencher<'_, '_, &Runtime>)
where
    P: CrtPolyParameters,
{
    let mut rng = rand::thread_rng();
    let ctx = CrtContext::gen().await;
    let power = PowerPoly::<P>::random(&mut rng);
    b.iter(|| async {
        let mut crt = CrtPoly::new();
        crt.clone_from_power(&ctx, black_box(&power)).await;
    })
}
