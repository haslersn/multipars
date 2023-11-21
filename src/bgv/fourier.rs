use std::mem;

use super::residue::vec::GenericResidueVec;

pub async fn fast_fourier_transform<ResidueVec>(
    root_powers: &ResidueVec,
    inverse: bool,
    mut input: ResidueVec,
) -> ResidueVec
where
    ResidueVec: GenericResidueVec,
{
    let n = input.len();
    debug_assert!(n >= 2);
    debug_assert!(n.count_ones() == 1);

    let mut output = ResidueVec::new(n);
    for shift in 0..n.trailing_zeros() {
        let size = 1 << shift;
        let count = n >> (shift + 1);
        for i in 0..count {
            for j in 0..size {
                let lhs = input[size * i + j];
                let mut rhs = input[size * i + j + n / 2];
                if j != 0 {
                    let root_power_index = if inverse {
                        count * (n - j) % n
                    } else {
                        count * j % n
                    };
                    rhs *= root_powers[root_power_index];
                }
                output[size * (2 * i) + j] = lhs + rhs;
                output[size * (2 * i + 1) + j] = lhs - rhs;
            }
        }
        mem::swap(&mut output, &mut input);
        tokio::task::yield_now().await;
    }

    input
}

#[cfg(test)]
mod tests {
    use crypto_bigint::Random;

    use crate::bgv::{
        fourier::fast_fourier_transform,
        params::ToyCipher,
        poly::{crt::CrtPolyParameters, CrtContext, CrtStrategy, PolyParameters},
        residue::vec::GenericResidueVec,
    };

    #[tokio::test]
    async fn dft_roundtrip() {
        if let CrtStrategy::Fourier = ToyCipher::CRT_STRATEGY {
            let ctx = if let CrtContext::Fourier(ctx) = CrtContext::<ToyCipher>::gen().await {
                ctx
            } else {
                panic!("created context that is incompatible")
            };
            let mut rng = rand::thread_rng();
            let mut input = <ToyCipher as PolyParameters>::Vec::new(ctx.dft_size);
            for entry in input.iter_mut() {
                *entry = Random::random(&mut rng);
            }
            let output = fast_fourier_transform(&ctx.dft_root_powers, false, input.clone()).await;
            let mut input_roundtrip =
                fast_fourier_transform(&ctx.dft_root_powers, true, output).await;
            for entry in input_roundtrip.iter_mut() {
                *entry *= &ctx.dft_size_inverse;
            }
            assert_eq!(input, input_roundtrip);
        } else {
            panic!("ToyCipher doesn't use DFT");
        }
    }

    #[tokio::test]
    async fn dft_convolution() {
        if let CrtStrategy::Fourier = ToyCipher::CRT_STRATEGY {
            let ctx = if let CrtContext::Fourier(ctx) = CrtContext::<ToyCipher>::gen().await {
                ctx
            } else {
                panic!("created context that is incompatible")
            };
            let mut rng = rand::thread_rng();
            let mut input1 = <ToyCipher as PolyParameters>::Vec::new(ctx.dft_size);
            let mut input2 = <ToyCipher as PolyParameters>::Vec::new(ctx.dft_size);
            for entry in input1.iter_mut() {
                *entry = Random::random(&mut rng);
            }
            for entry in input2.iter_mut() {
                *entry = Random::random(&mut rng);
            }

            // Naively compute convolution result
            let mut naively_convoluted = <ToyCipher as PolyParameters>::Vec::new(ctx.dft_size);
            for n in 0..ctx.dft_size {
                for m in 0..ctx.dft_size {
                    naively_convoluted[n] +=
                        input1[m] * input2[(n + ctx.dft_size - m) % ctx.dft_size];
                }
            }

            // Compute convolution via FFT
            let mut output1 = fast_fourier_transform(&ctx.dft_root_powers, false, input1).await;
            let output2 = fast_fourier_transform(&ctx.dft_root_powers, false, input2).await;
            for (dst, src) in output1.iter_mut().zip(output2.iter()) {
                *dst *= *src; // TODO: Can we support references on the RHS, too?
            }
            let mut convoluted = fast_fourier_transform(&ctx.dft_root_powers, true, output1).await;
            for entry in convoluted.iter_mut() {
                *entry *= &ctx.dft_size_inverse;
            }

            assert_eq!(convoluted, naively_convoluted);
        } else {
            panic!("ToyCipher doesn't use DFT");
        }
    }
}
