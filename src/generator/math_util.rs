/// Calculate L from K as per RFC 5053 Section 5.4.2.3
///
/// # Algorithm
///
/// 1. Calculate X: smallest integer where X*(X-1) >= 2*K
/// 2. Calculate S: smallest prime >= ceil(0.01*K) + X
/// 3. Calculate H: smallest H where C(H, ceil(H/2)) >= K + S
/// 4. L = K + S + H
///
/// # Note
///
/// This uses the helper functions from triple_generator module

/// Calculate the smallest positive integer X such that X*(X-1) >= 2*K
/// not the index X, but the value X
pub(crate) fn calculate_x_val(k: u32) -> u32 {
    let mut x = 1;
    while x * (x - 1) < 2 * k {
        x += 1;
    }
    x
}

/// Calculate S for RFC 5053: smallest prime >= ceil(0.01*K) + X
pub(crate) fn calculate_s(k: u32, x: u32) -> u32 {
    let threshold = ((k as f64 * 0.01).ceil() as u32) + x;
    next_prime(threshold)
}

/// Calculate H for RFC 5053: smallest H where C(H, ceil(H/2)) >= K + S
pub(crate) fn calculate_h(k: u32, s: u32) -> u32 {
    let target = k + s;
    let mut h = 1;
    
    loop {
        let h_half = (h as f64 / 2.0).ceil() as u32;
        if binomial(h, h_half) >= target as u64 {
            break;
        }
        h += 1;
    }
    
    h
}

/// Calculate L = K + S + H
pub(crate) fn calculate_l(k: u32) -> u32 {
    let x_val = calculate_x_val(k);
    let s = calculate_s(k, x_val);
    let h = calculate_h(k, s);
    // println!("S, H, L are {}, {}, {}", s, h, k + s + h);
    return k + s + h;
}

pub(crate) fn calculate_l_with_params(k: u32, s: u32, h: u32) -> u32 {
    k + s + h
}

/// Find the smallest prime number >= n
///
/// Uses trial division to check primality.
///
/// # Arguments
///
/// * `n` - Lower bound
///
/// # Returns
///
/// The smallest prime >= n
pub(crate) fn next_prime(n: u32) -> u32 {
    let mut candidate = n;
    while !is_prime(candidate) {
        candidate += 1;
    }
    candidate
}

/// Check if a number is prime
pub(crate) fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    
    let sqrt_n = (n as f64).sqrt() as u32;
    for i in (3..=sqrt_n).step_by(2) {
        if n % i == 0 {
            return false;
        }
    }
    true
}

/// Calculate binomial coefficient C(n, k) = n! / (k! * (n-k)!)
///
/// # Arguments
///
/// * `n` - Upper value
/// * `k` - Lower value
///
/// # Returns
///
/// Binomial coefficient C(n, k)
pub(crate) fn binomial(n: u32, k: u32) -> u64 {
    if k > n {
        return 0;
    }
    if k == 0 || k == n {
        return 1;
    }
    
    let k = k.min(n - k); // Take advantage of symmetry
    let mut result: u64 = 1;
    
    for i in 0..k {
        result = result * (n - i) as u64 / (i + 1) as u64;
    }
    
    result
}