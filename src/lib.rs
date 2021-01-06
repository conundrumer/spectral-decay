#![allow(incomplete_features)]
#![feature(generic_associated_types)]

mod ring_buffer;
mod fft_sizes;
mod spectral_decay;

pub use crate::spectral_decay::{
    SpectralDecay,
    SpectralDecayParameters
};

use serde::{Serialize, Deserialize};

use baseplug::{
    ProcessContext,
    Plugin,
};

baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct SpectralModel {
        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Fuzz")]
        #[unsmoothed]
        fuzz: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Loss")]
        #[unsmoothed]
        loss: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Glitch frequency")]
        #[unsmoothed]
        glitch_freq: f32,

        #[model(min = 1.0, max = 100.0)]
        #[parameter(name = "Glitch gain", unit = "Decibels",
            gradient = "Exponential")]
        #[unsmoothed]
        glitch_gain: f32,

        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Grain size")]
        #[unsmoothed]
        grain_select: f32,

        // modulating "Grain size" changes the amount of delay, which causes time stretching
        // to avoid time stretching, set "Delay compensation" to the max value of "Grain size"
        #[model(min = 0.0, max = 1.0)]
        #[parameter(name = "Delay compensation")]
        #[unsmoothed]
        delay_select: f32,
    }
}

impl Into<SpectralDecayParameters> for &SpectralModelProcess<'_> {
    fn into(self) -> SpectralDecayParameters {
        SpectralDecayParameters {
            grain_select: *self.grain_select,
            fuzz: *self.fuzz,
            loss: *self.loss,
            glitch_freq: *self.glitch_freq,
            glitch_gain: *self.glitch_gain,
            delay_select: *self.delay_select
        }
    }
}

impl Default for SpectralModel {
    fn default() -> Self {
        Self {
            grain_select: 0.5,
            fuzz: 0.0,
            loss: 0.5,
            glitch_freq: 0.1,
            glitch_gain: 100.,
            delay_select: 0.0
        }
    }
}

struct SpectralPlugin {
    sd: [SpectralDecay; 2]
}

impl Plugin for SpectralPlugin {
    const NAME: &'static str = "Spectral Decay";
    const PRODUCT: &'static str = "Spectral Decay";
    const VENDOR: &'static str = "Conundrumer";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = SpectralModel;

    #[inline]
    fn new(_sample_rate: f32, _model: &SpectralModel) -> Self {
        let grain_sizes = &fft_sizes::generate_sizes(64, 8192, 9);
        Self {
            sd: [SpectralDecay::new(grain_sizes), SpectralDecay::new(grain_sizes)]
        }
    }

    #[inline]
    fn process(&mut self, model: &SpectralModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;
        let params = model.into();

        self.sd[0].set_params(params);
        self.sd[1].set_params(params);

        self.sd[0].process(input[0], output[0]);
        self.sd[1].process(input[1], output[1]);
    }
}

// commment this out to test and run examples
baseplug::vst2!(SpectralPlugin, b"SpDc");