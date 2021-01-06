// in order to smoothly change grain size, we want logarithmically spaced FFT sizes
// they should be closely spaced so that there aren't any jumps in grain size when changing
// but also we want the sizes to be easy to FFT, so pick the prime factors carefully
// RustFFT loves lots of 2's and 3's and can tolerate some 5, 7, 11

pub fn generate_sizes(start: u64, end: u64, div_per_oct: u64) -> Vec<usize> {
    let mut candidates = Vec::new();

    let startf = start as f64;
    let endf = end as f64;
    let div_per_oct = div_per_oct as f64;
    let num_sizes = ((endf.log2() - startf.log2()) * div_per_oct) as usize + 1;

    let primes = &[1, 5, 7, 11];
    // mandate it is divisible by 4
    for f2 in 2..=endf.log2().ceil() as u32 {
        let f2 = 2u64.pow(f2);
        for f3 in 0..=endf.log(3.).ceil() as u32 {
            let f3 = 3u64.pow(f3);
            for i in 0..primes.len() {
                let p1 = primes[i];
                for j in i..primes.len() {
                    let p2 = primes[j];

                    let x = f2 * f3 * p1 * p2;

                    if x >= start && x <= end {
                        candidates.push(x);
                    }
                }
            }
        }
    }
    candidates.sort();
    // dbg!(&candidates);
    let candidates_log: Vec<_> = candidates.iter().map(|&x| (x as f64).log2()).collect();

    (0..num_sizes).map(|i| {
        let ideal = startf.log2() + i as f64 / div_per_oct;

        let (index, _c) = candidates_log.iter().enumerate().min_by(|(_, &x), (_, &y)| {
            (x - ideal).abs().partial_cmp(&(y - ideal).abs()).unwrap()
        }).unwrap();

        // println!("{:.2}", (_c - ideal).abs());

        candidates[index] as usize
    }).collect()
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fft_sizes() {
        let sizes = generate_sizes(8, 32, 2);
        assert_eq!(sizes[..5], [8, 12, 16, 24, 32]);

        println!("3");
        let _s = generate_sizes(64, 8192, 3);
        println!("4");
        let _s = generate_sizes(64, 8192, 4);
        println!("5");
        let _s = generate_sizes(64, 8192, 5);
        println!("7");
        let _s = generate_sizes(64, 8192, 7);
        println!("9");
        let _s = generate_sizes(64, 8192, 9);
        println!("11");
        let _s = generate_sizes(64, 8192, 11);

        // dbg!(_s);
        // panic!("###################")
    }
}