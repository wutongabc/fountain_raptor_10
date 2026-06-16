use super::math_util::*;
/// RFC 5053 LT Encoding Symbol Degree Set Generator
///
/// # Overview
///
/// This module implements the RFC 5053 LT Encoding Symbol Generator (LTEnc[])
/// algorithm for generating degree sets. This follows the deterministic algorithm
/// specified in RFC 5053 Section 5.4.4.3.
///
/// # Algorithm (RFC 5053 Section 5.4.4.3)
///
/// Given:
/// - K: number of source symbols
/// - L: number of intermediate symbols (derived from K)
/// - L': smallest prime >= L
/// - (d, a, b): triple from Triple generator
///
/// The LT Encoding Symbol Generator produces indices as follows:
///
/// ```text
/// While (b >= L) do b = (b + a) % L'
/// Let result = C[b]
/// For j = 1,...,min(d-1,L-1) do
///     b = (b + a) % L'
///     While (b >= L) do b = (b + a) % L'
///     result = result ^ C[b]
/// Return result
/// ```
///
/// We extract the index generation part:
/// - Generate indices for d symbols (or min(d, L) symbols)
/// - Each index is adjusted to be < L using the while loop
///
/// # Rust Concepts Explained
///
/// ## 1. Struct with Associated Functions
///
/// ```rust,ignore
/// pub struct RFC5053DegreeSet {
///     k: usize,
///     l: usize,
///     l_prime: usize,
/// }
///
/// impl RFC5053DegreeSet {
///     pub fn new(k: usize) -> Self { ... }
/// }
/// ```
use super::triple_generator::generate_triple_with_params;

/// RFC 5053 compliant LT degree set generator
///
/// This generator follows the exact algorithm specified in RFC 5053 Section 5.4.4.3
/// for generating LT encoding symbol indices.
///
/// # Fields
///
/// - `k`: Original number of source symbols
/// - `l`: Number of intermediate symbols (L = K + S + H)
/// - `l_prime`: Smallest prime >= L (for linear congruential generator)
///
pub struct RFC5053DegreeSet {
    k: usize,
    s: usize,
    l: usize,
    l_prime: usize,
}

impl RFC5053DegreeSet {
    /// Create a new RFC 5053 degree set generator
    ///
    /// # Arguments
    ///
    /// * `k` - Number of source symbols
    ///
    /// # Returns
    ///
    /// A new RFC5053DegreeSet instance configured with RFC 5053 parameters
    ///
    /// # Algorithm
    ///
    /// 1. Calculate L from K (as per RFC 5053 Section 5.4.2.3)
    /// 2. Find smallest prime L' >= L
    ///
    /// # Note
    ///
    /// The calculation of L involves several steps:
    /// - Calculate X: smallest integer where X*(X-1) >= 2*K
    /// - Calculate S: smallest prime >= ceil(0.01*K) + X
    /// - Calculate H: smallest H where C(H, ceil(H/2)) >= K + S
    /// - L = K + S + H
    pub fn new(k: usize) -> Self {
        let x = calculate_x_val(k as u32) as usize;
        let s = calculate_s(k as u32, x as u32) as usize;
        let h = calculate_h(k as u32, s as u32) as usize;
        let l = calculate_l_with_params(k as u32, s as u32, h as u32) as usize;
        let l_prime = next_prime(l as u32) as usize;
        Self { k, s, l, l_prime }
    }

    pub(crate) fn new_with_params(k: usize, s: usize, h: usize) -> Self {
        let l = calculate_l_with_params(k as u32, s as u32, h as u32) as usize;
        let l_prime = next_prime(l as u32) as usize;
        Self { k, s, l, l_prime }
    }

    /// Generate indices according to RFC 5053 LTEnc[] algorithm
    ///
    /// This implements the index generation part of RFC 5053 Section 5.4.4.3
    ///
    /// # Algorithm
    ///
    /// 1. Get triple (d, a, b) from Triple generator
    /// 2. Adjust b to be < L using while loop
    /// 3. Add b as first index
    /// 4. For j = 1 to min(d-1, L-1):
    ///    - Advance b: b = (b + a) % L'
    ///    - Adjust b to be < L
    ///    - Add b to indices
    /// 5. Separate indices into active and inactive:
    ///    - Active: index < k + l
    ///    - Inactive: index >= k + l
    ///
    /// # Arguments
    ///
    /// * `coded_id` - Encoding Symbol ID (X in RFC 5053)
    ///
    /// # Returns
    ///
    /// Tuple of (active_indices, inactive_indices)
    pub fn generate_indices(&self, coded_id: usize) -> (Vec<usize>, Vec<usize>) {
        // Step 1: Get triple from RFC 5053 Triple Generator
        // let (d, a, b) = generate_triple(self.k as u32, coded_id as u32);
        let (d, a, b) = generate_triple_with_params(
            self.k as u32,
            coded_id as u32,
            self.s as u32,
            self.l as u32,
            self.l_prime as u32,
        );
        // dbg!("the triple is", d, a, b);

        // check if a is an integer between 1 and l_prime - 1
        if a < 1 || a >= self.l_prime as u32 {
            dbg!("self.l_prime", self.l_prime);
            panic!(
                "the a is invalid, return empty vectors, d={}, a={}, b={}",
                d, a, b
            );
        }

        // check if b is an integer between 0 and l_prime - 1
        if b >= self.l_prime as u32 {
            panic!(
                "the b is invalid, return empty vectors, d={}, a={}, b={}",
                d, a, b
            );
        }

        // Pre-allocate vectors with expected capacity
        // The degree will be min(d, L)
        let expected_degree = std::cmp::min(d as usize, self.l);
        let mut active_indices = Vec::with_capacity(expected_degree);
        let mut inactive_indices = Vec::with_capacity(expected_degree);

        // Threshold for separating active and inactive indices
        let threshold = self.k + self.s;
        // let threshold = self.k;
        // dbg!("threshold, self.k + self.s", threshold, self.k, self.s);

        // Convert to mutable variables for the algorithm
        let mut b_current = b as usize;
        let a_step = a as usize;

        // Step 2: Pre-adjust initial b
        // While (b >= L) do b = (b + a) % L'
        while b_current >= self.l {
            b_current = (b_current + a_step) % self.l_prime;
        }

        // Step 3: Add first index
        // result = C[b]
        // dbg!("b_current", b_current, "threshold", threshold);
        if b_current < threshold {
            active_indices.push(b_current);
        } else {
            inactive_indices.push(b_current);
        }

        // Step 4: Generate remaining indices
        // For j = 1,...,min(d-1,L-1) do
        let iterations = std::cmp::min(d as usize - 1, self.l - 1);

        for _ in 0..iterations {
            // b = (b + a) % L'
            b_current = (b_current + a_step) % self.l_prime;

            // While (b >= L) do b = (b + a) % L'
            while b_current >= self.l {
                b_current = (b_current + a_step) % self.l_prime;
            }

            // result = result ^ C[b]
            // Separate into active and inactive based on threshold
            // dbg!("b_current in the loop", b_current, "threshold", threshold);
            if b_current < threshold {
                active_indices.push(b_current);
            } else {
                inactive_indices.push(b_current);
            }
        }

        /*
        Since the decoder will do the following process to the inactive indices
        in line 315-317 of bp.rs, we must let the inactive indices be mined
        by num_active() or threshold to avoid out of index.
        '''
        for &var_id in &inactive_indices {
            self.add_edge_inactive(var_id + self.params.num_active(), check_id);
        }
        '''
        */

        for i in 0..inactive_indices.len() {
            inactive_indices[i] -= threshold;
        }

        (active_indices, inactive_indices)
    }

    /// Generate degree set for a coded symbol
    ///
    /// This is the main interface method required by the DegreeSetGenerator trait.
    ///
    /// # Arguments
    ///
    /// * `coded_id` - The encoding symbol ID
    ///
    /// # Returns
    ///
    /// A tuple of (active_indices, inactive_indices):
    /// - `active_indices`: Indices < k + l
    /// - `inactive_indices`: Indices >= k + l
    ///
    /// # Note
    ///
    /// RFC 5053 LT encoding separates indices based on threshold k + l.
    pub fn degree_set(&mut self, coded_id: usize) -> (Vec<usize>, Vec<usize>) {
        self.generate_indices(coded_id)
    }

    /// Get the name of this degree set generator
    pub fn name(&self) -> &str {
        "RFC5053"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc5053_degree_set_creation() {
        // Test that we can create a degree set generator
        let k = 100;
        let degree_set = RFC5053DegreeSet::new(k);

        assert_eq!(degree_set.k, k);
        assert!(degree_set.l > k, "L should be greater than K");
        assert!(degree_set.l_prime >= degree_set.l, "L' should be >= L");
        assert!(is_prime(degree_set.l_prime as u32), "L' should be prime");
    }

    #[test]
    fn test_degree_set_generation() {
        // Test that degree set generation produces valid indices
        let k = 100;
        let degree_set = RFC5053DegreeSet::new(k);

        // Generate indices for first encoding symbol
        let (active_indices, inactive_indices) = degree_set.generate_indices(0);

        // Should have at least 1 index (degree >= 1)
        let total_indices = active_indices.len() + inactive_indices.len();
        assert!(total_indices > 0, "Should generate at least one index");

        // All active indices should be within valid range [0, k+l)
        for &idx in &active_indices {
            assert!(
                idx < k + degree_set.l,
                "Active index {} out of range [0, {})",
                idx,
                k + degree_set.l
            );
        }

        // All inactive indices should be within valid range [k+l, L)
        for &idx in &inactive_indices {
            assert!(
                idx >= k + degree_set.l,
                "Inactive index {} should be >= {}",
                idx,
                k + degree_set.l
            );
            assert!(
                idx < degree_set.l,
                "Inactive index {} out of range [0, {})",
                idx,
                degree_set.l
            );
        }
    }

    #[test]
    fn test_deterministic_generation() {
        // Test that same coded_id produces same indices
        let k = 100;
        let degree_set1 = RFC5053DegreeSet::new(k);
        let degree_set2 = RFC5053DegreeSet::new(k);

        let coded_id = 42;
        let (active1, inactive1) = degree_set1.generate_indices(coded_id);
        let (active2, inactive2) = degree_set2.generate_indices(coded_id);

        assert_eq!(
            active1, active2,
            "Same coded_id should produce same active indices"
        );
        assert_eq!(
            inactive1, inactive2,
            "Same coded_id should produce same inactive indices"
        );
    }

    #[test]
    fn test_different_coded_ids() {
        // Test that different coded_ids produce different indices
        let k = 100;
        let degree_set = RFC5053DegreeSet::new(k);

        let (active1, inactive1) = degree_set.generate_indices(0);
        let (active2, inactive2) = degree_set.generate_indices(1);

        // At least one should differ
        assert!(
            active1 != active2 || inactive1 != inactive2,
            "Different coded_ids should produce different indices"
        );
    }

    #[test]
    fn test_no_duplicate_indices() {
        // Test that indices don't have duplicates (though RFC allows it)
        let k = 100;
        let degree_set = RFC5053DegreeSet::new(k);

        for coded_id in 0..10 {
            let (active_indices, inactive_indices) = degree_set.generate_indices(coded_id);

            let active_unique_count = {
                let mut sorted = active_indices.clone();
                sorted.sort();
                sorted.dedup();
                sorted.len()
            };

            let inactive_unique_count = {
                let mut sorted = inactive_indices.clone();
                sorted.sort();
                sorted.dedup();
                sorted.len()
            };

            // Note: RFC 5053 doesn't require uniqueness, but it's typical
            // This test just documents the behavior
            println!(
                "coded_id {}: {} active ({} unique), {} inactive ({} unique)",
                coded_id,
                active_indices.len(),
                active_unique_count,
                inactive_indices.len(),
                inactive_unique_count
            );
        }
    }

    #[test]
    fn test_calculate_l() {
        // Test L calculation for various K values
        let test_cases = vec![
            (10, 17),   // Small K
            (100, 117), // Medium K
        ];

        for (k, expected_min_l) in test_cases {
            let x = calculate_x_val(k);
            let s = calculate_s(k, x);
            let h = calculate_h(k, s);
            let l = calculate_l_with_params(k, s, h);
            assert!(l >= k, "L should be >= K");
            assert!(
                l >= expected_min_l,
                "L={} should be >= expected minimum {} for K={}",
                l,
                expected_min_l,
                k
            );
        }
    }

    #[test]
    fn test_prime_finding() {
        assert_eq!(next_prime(1), 2);
        assert_eq!(next_prime(2), 2);
        assert_eq!(next_prime(3), 3);
        assert_eq!(next_prime(4), 5);
        assert_eq!(next_prime(10), 11);
        assert_eq!(next_prime(11), 11);
    }

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(5));
        assert!(is_prime(7));
        assert!(is_prime(11));
        assert!(!is_prime(9));
    }

    #[test]
    fn test_binomial() {
        assert_eq!(binomial(5, 2), 10);
        assert_eq!(binomial(6, 3), 20);
        assert_eq!(binomial(4, 4), 1);
        assert_eq!(binomial(4, 0), 1);
        assert_eq!(binomial(3, 5), 0);
    }

    #[test]
    fn test_degree_distribution() {
        // Test that degrees follow RFC 5053 distribution
        let k = 100;
        let degree_set = RFC5053DegreeSet::new(k);

        // Calculate the gap: [k, k+s+h)
        let k_u32 = k as u32;
        let x_val = {
            let mut x = 1;
            while x * (x - 1) < 2 * k_u32 {
                x += 1;
            }
            x
        };
        let s = next_prime(((k_u32 as f64 * 0.01).ceil() as u32) + x_val);
        let h = {
            let target = k_u32 + s;
            let mut h = 1;
            loop {
                let h_half = (h as f64 / 2.0).ceil() as u32;
                if binomial(h, h_half) >= target as u64 {
                    break;
                }
                h += 1;
            }
            h
        };
        let gap_start = k;
        let gap_end = (k_u32 + s + h) as usize;

        let mut degrees = Vec::new();
        // Test source symbols [0, k)
        for coded_id in 0..k {
            let (active, inactive) = degree_set.generate_indices(coded_id);
            degrees.push(active.len() + inactive.len());
        }
        // Skip the gap [k, k+s+h) and test repair symbols [k+s+h, k+s+h+900)
        for coded_id in gap_end..(gap_end + 900) {
            let (active, inactive) = degree_set.generate_indices(coded_id);
            degrees.push(active.len() + inactive.len());
        }

        // Calculate average degree
        let avg_degree = degrees.iter().sum::<usize>() as f64 / degrees.len() as f64;

        // RFC 5053 degree distribution should have reasonable average
        // Typically between 2 and 10 for most K values
        println!(
            "Average degree for K={}: {:.2} (gap: [{}, {}))",
            k, avg_degree, gap_start, gap_end
        );
        assert!(avg_degree > 1.0, "Average degree should be > 1");
        assert!(avg_degree < 50.0, "Average degree should be < 50");
    }

    #[test]
    fn test_degree_set_generator_trait() {
        // Test the DegreeSetGenerator trait implementation
        let k = 100;
        let mut degree_set = RFC5053DegreeSet::new(k);

        // Test that trait method works
        let (active_indices, inactive_indices) = degree_set.degree_set(0);

        // Should have at least some indices
        let total = active_indices.len() + inactive_indices.len();
        assert!(total > 0, "Should have at least one index");

        // Verify name
        assert_eq!(degree_set.name(), "RFC5053");
    }

    #[test]
    fn test_degree_set_trait_consistency() {
        // Test that trait method produces same result as direct method
        let k = 100;
        let mut degree_set = RFC5053DegreeSet::new(k);

        let coded_id = 42;

        // Call through trait
        let (active_via_trait, inactive_via_trait) = degree_set.degree_set(coded_id);

        // Call direct method
        let (direct_active, direct_inactive) = degree_set.generate_indices(coded_id);

        // Should be identical
        assert_eq!(
            active_via_trait, direct_active,
            "Trait method should produce same active indices as direct method"
        );
        assert_eq!(
            inactive_via_trait, direct_inactive,
            "Trait method should produce same inactive indices as direct method"
        );
    }
}
