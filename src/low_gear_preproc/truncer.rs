use crypto_bigint::U64;
use futures_util::{SinkExt, StreamExt};

use crate::{
    bgv::residue::native::GenericNativeResidue,
    bi_channel::BiChannel,
    connection::{Connection, StreamError},
};

pub struct Truncer<S>
where
    S: GenericNativeResidue,
{
    ch_a: BiChannel<Vec<S>>,
    mac_key: S,
}

impl<S> Truncer<S>
where
    S: GenericNativeResidue,
{
    pub async fn new(conn: &mut Connection, mac_key: S) -> Result<Self, StreamError> {
        Ok(Self {
            ch_a: BiChannel::open(conn).await?,
            mac_key,
        })
    }

    pub async fn truncate<K, KS, KSS, const PID: usize>(
        &mut self,
        wide_a: &[KSS],
        wide_a_tags: &[KSS],
        b: &[K],
        b_tags: &[KS],
        wide_c: &[KSS],
        wide_c_tags: &[KSS],
    ) -> (Vec<KS>, Vec<KS>, Vec<KS>, Vec<KS>)
    where
        K: GenericNativeResidue,
        KS: GenericNativeResidue,
        KSS: GenericNativeResidue,
    {
        let len = wide_a.len();
        // TODO: Check all lengths against len

        let a_mod2s: Vec<_> = wide_a.iter().copied().map(S::from_unsigned).collect();

        let (rx_a, tx_a) = self.ch_a.split();

        let (_, (a, a_tags, c, c_tags)) = tokio::join!(
            async {
                tx_a.send(a_mod2s.clone()).await.unwrap();
            },
            async {
                let remote_a_mod2s = rx_a.next().await.unwrap().unwrap();
                if remote_a_mod2s.len() != len {
                    // TODO: Error handling instead of panic
                    panic!("received a_mod2s has wrong length");
                }

                let sigma_a: Vec<_> = a_mod2s
                    .iter()
                    .zip(remote_a_mod2s.iter())
                    .map(|(l, r)| KS::from_unsigned(*l) + KS::from_unsigned(*r))
                    .collect();

                let hat_a_tags: Vec<_> = wide_a_tags
                    .iter()
                    .zip(sigma_a.iter())
                    .map(|(a, s)| *a - KSS::from_unsigned(*s) * KSS::from_unsigned(self.mac_key))
                    .collect();
                let hat_c: Vec<_> = wide_c
                    .iter()
                    .zip(sigma_a.iter())
                    .zip(b.iter())
                    .map(|((c, s), b)| *c - KSS::from_unsigned(*s) * KSS::from_unsigned(*b))
                    .collect();
                let hat_c_tags: Vec<_> = wide_c_tags
                    .iter()
                    .zip(sigma_a.iter())
                    .zip(b_tags.iter())
                    .map(|((c, s), b)| *c - KSS::from_unsigned(*s) * KSS::from_unsigned(*b))
                    .collect();

                let a = wide_a.iter().copied().map(shift).collect();
                let a_tags = hat_a_tags
                    .iter()
                    .copied()
                    .map(modified_shift::<_, _, PID>)
                    .collect();
                let c = hat_c
                    .iter()
                    .copied()
                    .map(modified_shift::<_, _, PID>)
                    .collect();
                let c_tags = hat_c_tags
                    .iter()
                    .copied()
                    .map(modified_shift::<_, _, PID>)
                    .collect();

                (a, a_tags, c, c_tags)
            }
        );

        (a, a_tags, c, c_tags)
    }
}

fn shift<KS, KSS>(x: KSS) -> KS
where
    KS: GenericNativeResidue,
    KSS: GenericNativeResidue,
{
    KS::from_unsigned(x.shr_vartime(KSS::BITS - KS::BITS))
}

fn modified_shift<KS, KSS, const PID: usize>(mut x: KSS) -> KS
where
    KS: GenericNativeResidue,
    KSS: GenericNativeResidue,
{
    if PID == 0 {
        x -= KSS::from_uint(U64::ONE);
    }
    let mut res = shift(x);
    if PID == 0 {
        res += KS::from_uint(U64::ONE);
    }
    res
}
