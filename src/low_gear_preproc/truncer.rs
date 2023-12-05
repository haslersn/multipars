use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    bgv::residue::native::GenericNativeResidue,
    bi_channel::BiChannel,
    connection::{Connection, StreamError},
};

#[derive(Clone, Deserialize, Serialize)]
struct ComMsg<S> {
    hat_a_tags_mod2s: Vec<S>,
    hat_c_mod2s: Vec<S>,
    hat_c_tags_mod2s: Vec<S>,
}

pub struct Truncer<S>
where
    S: GenericNativeResidue,
{
    ch_a: BiChannel<Vec<S>>,
    ch_com: BiChannel<ComMsg<S>>,
    mac_key: S,
}

impl<S> Truncer<S>
where
    S: GenericNativeResidue,
{
    pub async fn new(conn: &mut Connection, mac_key: S) -> Result<Self, StreamError> {
        Ok(Self {
            ch_a: BiChannel::open(conn, "Truncer:a").await?,
            ch_com: BiChannel::open(conn, "Truncer:com").await?,
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

        let (_, (_, (a, a_tags, c, c_tags))) = tokio::join!(
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

                let mut hat_a_tags: Vec<_> = wide_a_tags
                    .iter()
                    .zip(sigma_a.iter())
                    .map(|(a, s)| *a - KSS::from_unsigned(*s) * KSS::from_unsigned(self.mac_key))
                    .collect();
                let mut hat_c: Vec<_> = wide_c
                    .iter()
                    .zip(sigma_a.iter())
                    .zip(b.iter())
                    .map(|((c, s), b)| *c - KSS::from_unsigned(*s) * KSS::from_unsigned(*b))
                    .collect();
                let mut hat_c_tags: Vec<_> = wide_c_tags
                    .iter()
                    .zip(sigma_a.iter())
                    .zip(b_tags.iter())
                    .map(|((c, s), b)| *c - KSS::from_unsigned(*s) * KSS::from_unsigned(*b))
                    .collect();

                let com_msg = ComMsg::<S> {
                    hat_a_tags_mod2s: hat_a_tags.iter().map(|x| S::from_unsigned(*x)).collect(),
                    hat_c_mod2s: hat_c.iter().map(|x| S::from_unsigned(*x)).collect(),
                    hat_c_tags_mod2s: hat_c_tags.iter().map(|x| S::from_unsigned(*x)).collect(),
                };

                let (rx_com, tx_com) = self.ch_com.split();

                tokio::join!(
                    async {
                        tx_com.send(com_msg.clone()).await.unwrap();
                    },
                    async {
                        let remote_com = rx_com.next().await.unwrap().unwrap();
                        // TODO: Error handling instead of panic
                        if remote_com.hat_a_tags_mod2s.len() != len {
                            panic!("received hat_a_tags_mod2s has wrong length");
                        }
                        if remote_com.hat_c_mod2s.len() != len {
                            panic!("received hat_c_mod2s has wrong length");
                        }
                        if remote_com.hat_c_tags_mod2s.len() != len {
                            panic!("received hat_c_tags_mod2s has wrong length");
                        }

                        if PID == 0 {
                            for (dst, src) in hat_a_tags
                                .iter_mut()
                                .zip(remote_com.hat_a_tags_mod2s.iter())
                            {
                                *dst += KSS::from_unsigned(*src);
                                Self::check_is_zero_mod2s(*dst);
                            }
                            for (dst, src) in hat_c.iter_mut().zip(remote_com.hat_c_mod2s.iter()) {
                                *dst += KSS::from_unsigned(*src);
                                Self::check_is_zero_mod2s(*dst);
                            }
                            for (dst, src) in hat_c_tags
                                .iter_mut()
                                .zip(remote_com.hat_c_tags_mod2s.iter())
                            {
                                *dst += KSS::from_unsigned(*src);
                                Self::check_is_zero_mod2s(*dst);
                            }
                        } else {
                            for (l, r) in com_msg
                                .hat_a_tags_mod2s
                                .iter()
                                .zip(remote_com.hat_a_tags_mod2s.iter())
                            {
                                Self::check_is_zero_mod2s(
                                    KS::from_unsigned(*l) + KS::from_unsigned(*r),
                                );
                            }
                            for (l, r) in com_msg
                                .hat_c_mod2s
                                .iter()
                                .zip(remote_com.hat_c_mod2s.iter())
                            {
                                Self::check_is_zero_mod2s(
                                    KS::from_unsigned(*l) + KS::from_unsigned(*r),
                                );
                            }
                            for (l, r) in com_msg
                                .hat_c_tags_mod2s
                                .iter()
                                .zip(remote_com.hat_c_tags_mod2s.iter())
                            {
                                Self::check_is_zero_mod2s(
                                    KS::from_unsigned(*l) + KS::from_unsigned(*r),
                                );
                            }
                        }

                        println!("Trunc: check passed");

                        let a = wide_a.iter().copied().map(shift).collect();
                        let a_tags = hat_a_tags.iter().copied().map(shift).collect();
                        let c = hat_c.iter().copied().map(shift).collect();
                        let c_tags = hat_c_tags.iter().copied().map(shift).collect();

                        (a, a_tags, c, c_tags)
                    }
                )
            }
        );

        (a, a_tags, c, c_tags)
    }

    fn check_is_zero_mod2s(x: impl GenericNativeResidue) {
        // TODO: Error handling instead
        assert_eq!(S::from_unsigned(x), S::ZERO);
    }
}

fn shift<KS, KSS>(x: KSS) -> KS
where
    KS: GenericNativeResidue,
    KSS: GenericNativeResidue,
{
    KS::from_unsigned(x.shr_vartime(KSS::BITS - KS::BITS))
}
