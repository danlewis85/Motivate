// This is derivative work of https://athemathmo.github.io/2016/06/24/using-gmm-in-rust.html
// See - https://github.com/AtheMathmo/AtheMathmo.github.io/issues/2 for proof of license
//
//
// The MIT License (MIT) - for the original work
//
// Copyright (c) 2016 James Lucas
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights 
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell 
// copies of the Software, and to permit persons to whom the Software is furnished
// to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all 
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR 
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, 
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER 
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, 
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE 
// SOFTWARE.


use rand::distributions::{Normal, Distribution};
use rand::Rng;
use rand::thread_rng;

/// This gets samples from a Gaussian Mixture Model
/// * count: The number of samples
/// * distribution: A vec of tuples (mean, sd, weight)
/// * Returns: A vec of samples
pub fn get_samples_from_gmm(
    count: usize,
    distributions: Vec<(f64, f64, f64)>)
    -> Vec<f64>
{
    // Create the Normal distributions
    let gaussians: Vec<Normal> = distributions
        .iter()
        .map(|(mean, sd, _)| Normal::new(*mean, *sd))
        .collect();

    // Filter out just the weights
    let weights: Vec<f64> = distributions
        .iter()
        .map(|(_, _, weight)| *weight)
        .collect();

    // Create a random number generator    
    let mut rng = thread_rng();

    // Create a vec for the samples
    let mut samples = Vec::with_capacity(count);

    // Generate count samples
    for _ in 0..count {
        // Chose a distribution based upon its weight
        let chosen_guassian = gaussians[pick_distribution_id(&weights, &mut rng)];

        // Push a sample from that distribution to the samples vec
        samples.push(chosen_guassian.sample(&mut rng));

    }
    // Return the generated samples
    samples
}

/// Pick a distribution id to draw from
/// * weights: The weights of each distribution, where the index in weights corresponds
/// to the index of a distribution in another vec
fn pick_distribution_id<R: Rng>(weights: &[f64], rng: &mut R) -> usize {
    // Calculate the total of all the weights
    let total_weight = weights
        .iter()
        .sum();

    // Generate a number between 0 and total_weight
    let random_number = rng.gen_range(0.0f64, total_weight);

    // Initialise culmulative distribtuion function at 0
    let mut cdf = 0.0f64;
    // For each weight
    for (i, weight) in weights.iter().enumerate() {
        // Add weight to the cdf
        cdf += *weight;
        // If random number falls within the range of weight, return the index of this weight
        if random_number < cdf {
            return i;
        }
    }

    // This should never be reached
    panic!("No random value was sampled!")
}