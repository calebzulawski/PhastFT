#![feature(portable_simd)]
use crate::kernels::{fft_chunk_2, fft_chunk_4, fft_chunk_n, fft_chunk_n_simd, Float};
use crate::{cobra::cobra_apply, twiddles::generate_twiddles};

mod cobra;
mod kernels;
mod twiddles;
mod utils;

/// FFT -- Decimation in Frequency
///
/// This is just the decimation-in-time algorithm, reversed.
/// The inputs are in normal order, and the outputs are bit reversed.
///
/// # Panics
///
/// Panics if `reals.len() != imags.len()`
///
/// [1] https://inst.eecs.berkeley.edu/~ee123/sp15/Notes/Lecture08_FFT_and_SpectAnalysis.key.pdf
pub fn fft_dif(reals: &mut [Float], imags: &mut [Float]) {
    assert_eq!(reals.len(), imags.len());
    let n: usize = reals.len().ilog2() as usize;

    for t in (0..n).rev() {
        let dist = 1 << t;
        let chunk_size = dist << 1;

        if chunk_size > 4 {
            let (twiddles_re, twiddles_im) = generate_twiddles(dist);
            if chunk_size >= 16 {
                fft_chunk_n_simd(reals, imags, &twiddles_re, &twiddles_im, dist);
            } else {
                fft_chunk_n(reals, imags, &twiddles_re, &twiddles_im, dist);
            }
        } else if chunk_size == 2 {
            fft_chunk_2(reals, imags);
        } else if chunk_size == 4 {
            fft_chunk_4(reals, imags);
        }
    }

    if n < 22 {
        cobra_apply(reals, n);
        cobra_apply(imags, n);
    } else {
        std::thread::scope(|s| {
            s.spawn(|| cobra_apply(reals, n));
            s.spawn(|| cobra_apply(imags, n));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::assert_f64_closeness;
    use rustfft::{num_complex::Complex64, FftPlanner};
    use std::ops::Range;

    #[test]
    fn fft() {
        let range = Range { start: 2, end: 17 };

        for k in range {
            let n = 1 << k;

            let mut reals: Vec<Float> = (1..=n).map(|i| i as Float).collect();
            let mut imags: Vec<Float> = (1..=n).map(|i| i as Float).collect();
            fft_dif(&mut reals, &mut imags);

            let mut buffer: Vec<Complex64> = (1..=n)
                .map(|i| Complex64::new(f64::from(i), f64::from(i)))
                .collect();

            let mut planner = FftPlanner::new();
            let fft = planner.plan_fft_forward(buffer.len());
            fft.process(&mut buffer);

            reals
                .iter()
                .zip(imags.iter())
                .enumerate()
                .for_each(|(i, (z_re, z_im))| {
                    let expect_re = buffer[i].re;
                    let expect_im = buffer[i].im;
                    assert_f64_closeness(*z_re, expect_re, 0.001);
                    assert_f64_closeness(*z_im, expect_im, 0.001);
                });
        }
    }
}
