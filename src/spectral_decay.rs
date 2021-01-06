use crate::ring_buffer::RingBuffer;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;
use realfft::{ComplexToReal, RealToComplex};
use random_fast_rng::{FastRng, Random};
use std::f32::consts::PI;

#[derive(Copy, Clone)]
pub struct SpectralDecayParameters {
    pub grain_select: f32,
    pub fuzz: f32,
    pub loss: f32,
    pub glitch_freq: f32,
    pub glitch_gain: f32,
    pub delay_select: f32,
}

impl Default for SpectralDecayParameters {
    fn default() -> Self {
        Self {
            grain_select: 0.,
            fuzz: 0.,
            loss: 0.,
            glitch_freq: 0.,
            glitch_gain: 1.,
            delay_select: 0.
        }
    }
}

pub struct SpectralDecay {
    grain_index: usize,
    grain_size: usize,
    hop: usize,
    delay_comp: usize,
    offset: usize,
    grains: Vec<(Vec<f32>, RealToComplex<f32>, ComplexToReal<f32>)>,
    in_buf: RingBuffer<f32>,
    out_buf: RingBuffer<f32>,
    time_buf: Vec<f32>,
    freq_buf: Vec<Complex<f32>>,
    rng: FastRng,
    params: SpectralDecayParameters
}

impl SpectralDecay {
    pub fn new(grain_sizes: &[usize]) -> Self {
        assert!(grain_sizes.len() > 0);
        assert!(grain_sizes.iter().all(|n| n % 4 == 0));
        assert!(grain_sizes.windows(2).all(|n| n[0] <= n[1])); // allow duplicate grain sizes for even spacing
        let n_max = *grain_sizes.last().unwrap();
        Self {
            grain_index: 0,
            grain_size: grain_sizes[0],
            hop: grain_sizes[0] / 4,
            delay_comp: grain_sizes[0] * 5 / 4,
            offset: 0,
            grains: grain_sizes.iter().map(|&n| (
                (0..n).map(|x| 0.5 - 0.5 * (x as f32 * 2. * PI / n as f32).cos()).collect(),
                RealToComplex::<f32>::new(n).unwrap(),
                ComplexToReal::<f32>::new(n).unwrap()
            )).collect(),
            in_buf: RingBuffer::new(n_max, true),
            out_buf: RingBuffer::new(n_max / 4 * 5, true),
            time_buf: vec![0.; n_max],
            freq_buf: vec![Complex::zero(); n_max / 2 + 1],
            rng: FastRng::new(),
            params: Default::default()
        }
    }

    fn select_to_index(&self, select: f32) -> usize {
        let num_grains = self.grains.len();

        ((select * num_grains as f32) as usize).min(num_grains - 1)
    }

    pub fn delay(&self) -> usize {
        (self.grain_size + self.hop).max(self.delay_comp)
    }

    pub fn set_params(&mut self, params: SpectralDecayParameters) {
        if params.grain_select != self.params.grain_select {
            let grain_index = self.select_to_index(params.grain_select);

            if self.grain_index != grain_index {
                self.grain_index = grain_index;

                let prev_grain_size = self.grain_size as isize;
                self.grain_size = self.grains[grain_index].0.len();
                let grain_size = self.grain_size as isize;

                if (grain_size - prev_grain_size).abs() > grain_size.min(prev_grain_size) {
                    // differ by more than a factor of 2, reset
                    self.offset = 0;
                    self.hop = self.grain_size / 4;
                } else {
                    // closer than or equal to a factor of 2, interpolate
                    let hop_phase = self.offset as f32 / self.hop as f32;
                    self.hop = self.grain_size / 4;
                    self.offset = (hop_phase * self.hop as f32) as usize;
                }
            }
        }
        if params.delay_select != self.params.delay_select {
            let delay_index = self.select_to_index(params.delay_select);

            self.delay_comp = self.grains[delay_index].0.len() / 4 * 5;
        }
        self.params = params
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        use std::iter::once;
        assert_eq!(input.len(), output.len());

        let initial_hop = input.len().min(self.hop - self.offset);

        let (in_head, in_tail) = input.split_at(initial_hop);
        let (out_head, out_tail) = output.split_at_mut(initial_hop);

        let in_iter = once(in_head).chain(in_tail.chunks(self.hop));
        let out_iter = once(out_head).chain(out_tail.chunks_mut(self.hop));

        // hop
        for (in_chunk, out_chunk) in in_iter.zip(out_iter) {
            self.in_buf.copy_replace(Some(in_chunk), None);
            self.out_buf.copy_replace(None, Some(out_chunk));

            self.offset += in_chunk.len();

            if self.offset >= self.hop {
                self.offset -= self.hop;
                self.process_buffers();
            }
        }
    }

    fn process_buffers(&mut self) {
        let delay = self.delay();
        let (ref window, ref mut fft, ref mut ifft) = self.grains[self.grain_index];
        let mut time_buf = &mut self.time_buf[..self.grain_size];
        let mut freq_buf = &mut self.freq_buf[..self.grain_size / 2 + 1];
        // window/normalize input

        for ((y, x), w) in time_buf.iter_mut().zip(self.in_buf.iter(-(self.grain_size as isize))).zip(window) {
            *y = x * 2. * w;
        }

        // to freq domain
        fft.process(&mut time_buf, &mut freq_buf).unwrap();

        // process spectrum
        let SpectralDecayParameters {
            fuzz,
            loss,
            glitch_freq,
            glitch_gain,
            ..
        } = self.params;

        let rng = &mut self.rng;
        let mut rand = || { rng.gen::<u32>() as f32 / u32::MAX as f32 };
        let mut max_amp = 0.;

        for x in freq_buf.iter() {
            max_amp = x.norm().max(max_amp);
        }

        for x in freq_buf.iter_mut() {
            if rand() < glitch_freq / 8. {
                let k = rand();
                *x *= k * k * glitch_gain;
            } else if x.norm() / max_amp < loss {
                *x = Complex::zero();
            } else if fuzz > 0. {
                let (r, theta) = x.to_polar();
                let delta = 2. * PI * rand();

                *x = Complex::from_polar(r, theta + delta * fuzz);
            }
        }

        // to time domain
        ifft.process(&mut freq_buf, &mut time_buf).unwrap();

        // window/normalize output
        let mut max_amp = 1.;
        for (x, w) in time_buf.iter_mut().zip(window) {
            *x *= w / self.grain_size as f32;
            max_amp = x.abs().max(max_amp);
        }

        // overlap add
        for (y, x) in self.out_buf.iter_mut((delay - self.grain_size) as isize).zip(time_buf) {
            *y += *x / (max_amp * 1.5);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sd_identity() {
        let mut sd = SpectralDecay::new(&[32, 64]);
        // let mut sd = SpectralDecay::new(&[16, 32]);
        let mut test_fn = |n, assert| {
            let input = vec![1.; n];
            let mut output = vec![0.; n];

            sd.process(&input, &mut output);

            dbg!(&output);

            if assert {
                for x in &output {
                    assert!((*x - 1.).abs() < 1e-6);
                }
            }
        };
        println!("ramp up to 1.");
        test_fn(64, false);
        println!("test 1 buffer");
        test_fn(8, true);
        println!("test 2 buffers");
        test_fn(16, true);
        println!("test partial buffer");
        test_fn(4, true);
        println!("test partial buffer again");
        test_fn(2, true);
        println!("test overlapping buffer");
        test_fn(8, true);
        println!("test 2 buffers");
        test_fn(16, true);
        // assert!(false);
    }

    #[test]
    fn sd_delay() {
        let n = 32;
        let mut sd = SpectralDecay::new(&[n, 2 * n]);

        let mut input = vec![0.; 2 * n];
        let mut output = vec![0.; 2 * n];

        input[0] = 1.;
        sd.process(&input, &mut output);

        let index = output.iter().position(|&x| x == 1.).unwrap();

        assert_eq!(sd.delay(), 32 + 8);
        assert_eq!(index, 32 + 8);

        let mut sd = SpectralDecay::new(&[n, 2 * n]);
        let mut p = SpectralDecayParameters::default();
        p.delay_select = 1.;
        sd.set_params(p);

        let mut sd2 = SpectralDecay::new(&[2 * n]);

        let mut input = vec![0.; 3 * n];
        let mut output = vec![0.; 3 * n];

        let mut input2 = vec![0.; 3 * n];
        let mut output2 = vec![0.; 3 * n];

        input[0] = 1.;
        sd.process(&input, &mut output);

        input2[0] = 1.;
        sd2.process(&input2, &mut output2);

        let index = output.iter().position(|&x| x == 1.).unwrap();

        let index2 = output2.iter().position(|&x| x == 1.).unwrap();

        assert_eq!(sd2.delay(), 64 + 16);
        assert_eq!(index2, 64 + 16);

        assert_eq!(sd.delay(), 64 + 16);
        assert_eq!(index, 64 + 16);
    }
}
