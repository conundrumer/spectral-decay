use spectral_decay::{
    SpectralDecay,
    SpectralDecayParameters
};

const AMEN: &[u8] = include_bytes!("amen.raw");

fn main() {
    use std::io::{self, Write};

    let wav = unsafe {
        // I hope you're on a little endian machine
        std::mem::transmute::<&[u8], &[i16]>(AMEN)
    };

    let input: Vec<_> = wav.iter().map(|x| *x as f32 / (i16::MAX - 1) as f32).collect();
    let mut output = vec![0f32; AMEN.len()];

    let mut sp = SpectralDecay::new(&[
        64,
        64 * 3/2,
        128,
        128 * 3/2,
        256,
        256 * 3/2,
        512,
        512 * 3/2,
        1024,
        1024 * 3/2,
        2048,
        2048 * 3/2,
        4096,
        4096 * 3/2,
        8192,
        ]);

    let automation = [
        // (1.0, (0., 0., 0., 0., 0.)),
        // (2.0, (1., 0., 0., 0., 0.)),
        // (3.0, (0., 0., 0., 0., 0.)),
        // (4.0, (1., 0., 0., 0., 0.)),

        (0.5, (0., 0., 0., 0., 1.)),
        (1.0, (1., 0., 0., 0., 1.)),
        (1.5, (0., 0.7, 0., 0., 1.)),
        (2.0, (1., 0.7, 0., 0., 1.)),
        (2.5, (0., 1., 0., 0., 1.)),
        (3.0, (1., 1., 0., 0., 1.)),
        (3.5, (0., 1., 0.1, 0., 1.)),
        (4.0, (1., 1., 0.1, 0., 1.)),
        (4.5, (0., 1., 0.5, 0., 1.)),
        (5.0, (1., 1., 0.5, 0., 1.)),
        (5.5, (0., 1., 1., 0.1, 100.)),
        (6.0, (1., 1., 1., 0.1, 100.)),
        (6.5, (0., 1., 1., 1., 50.)),
        (7.0, (1., 1., 1., 1., 50.)),
    ];
    let mut start = 0;
    for &(end, (_gs, p, l, gf, gg)) in &automation {
        let gs = end / 7.0;
        let end = (end * 44100.) as usize;
        sp.set_params(SpectralDecayParameters {
            grain_select: gs,
            fuzz: p,
            loss: l,
            glitch_freq: gf,
            glitch_gain: gg,
        });
        sp.process(&input[start..end], &mut output[start..end]);
        start = end;
    }

    let output_int: Vec<_> = output.iter().map(|&x| (x.clamp(-1., 1.) * (i16::MAX - 1) as f32) as i16).collect();

    let bytes = unsafe {
        std::mem::transmute::<&[i16], &[u8]>(&output_int)
    };

    // play this using sox or aplayer, e.g.
    // cargo run --example demo | sox -traw -r44100 -b16 -e signed-integer - -tcoreaudio
    io::stdout().write_all(bytes).unwrap();
}
