
/// *Raptor10 Systematic Code (RFC 5053)*
/// 
/// This module implements systematic fountain codes using Raptor10 (RFC 5053) 
/// parameters and degree set generator.
/// 
/// # Overview
/// 
/// Raptor10 is a rateless erasure code specified in RFC 5053, which is simpler
/// than RaptorQ (RFC 6330) but still provides excellent performance for many applications.
/// 
/// # Features
/// 
/// - Uses RFC 5053 degree set generator
/// - Uses HDPC10 precode from fountain_basic
/// - Uses default LDPC precode
/// - Supports systematic encoding

use fountain_engine::traits::{CodeScheme, LDPC, HDPC};
use fountain_engine::types::{CodeParams, CodeType, DecodingConfig, SubstitutionMethod};
use fountain_scheme::precodes::hdpc_binary::R10HDPC;
use fountain_scheme::precodes::ldpc::R10LDPC;
use crate::generator::rfc5053_degree_set::RFC5053DegreeSet;

/// Raptor10 Systematic Code (RFC 5053)
/// 
/// This struct provides a systematic fountain code implementation using Raptor10
/// parameters as specified in RFC 5053.
/// 
/// # Components
/// 
/// - **Degree Set**: RFC 5053 LT encoding symbol generator
/// - **HDPC**: HDPC10 from fountain_basic
/// - **LDPC**: Configurable LDPC type (default: ReversedLDPC)
/// 
/// # Parameters
/// 
/// Raptor10 calculates intermediate parameters L, S, H from the number of source symbols K:
/// - K: Number of source symbols
/// - L = K + S + H: Number of intermediate symbols
/// - S: LDPC symbols (smallest prime >= ceil(0.01*K) + X)
/// - H: HDPC symbols (smallest H where C(H, ceil(H/2)) >= K + S)
#[derive(Clone)]
pub struct Raptor10SysCode {
    params: CodeParams,
    //ldpc_creator: Box<dyn Fn(&CodeParams) -> Box<dyn LDPC>>,
    k: usize, // Store k for RFC5053DegreeSet creation
}

impl Raptor10SysCode {
    /// Create a new Raptor10 Systematic Code
    /// 
    /// # Arguments
    /// 
    /// * `k` - Number of source symbols
    /// * `_dmax` - Maximum degree for LT encoding (not used but kept for API compatibility)
    /// * `ldpc_type` - LDPC type
    /// 
    /// # Returns
    /// 
    /// A new `Raptor10SysCode` instance
    /// 
    /// # Algorithm
    /// 
    /// Calculates RFC 5053 parameters:
    /// 1. Calculate X: smallest integer where X*(X-1) >= 2*K
    /// 2. Calculate S: smallest prime >= ceil(0.01*K) + X
    /// 3. Calculate H: smallest H where C(H, ceil(H/2)) >= K + S
    /// 4. L = K + S + H (total intermediate symbols)
    pub fn new(k: usize, _dmax: usize) -> Self {
        // Calculate RFC 5053 parameters
        let k_u32 = k as u32;
        
        // Calculate X: smallest X where X*(X-1) >= 2*K
        let x_val = calculate_x_val(k_u32);
        
        // Calculate S: smallest prime >= ceil(0.01*K) + X
        let s = calculate_s(k_u32, x_val);
        
        // Calculate H: smallest H where C(H, ceil(H/2)) >= K + S
        let h = calculate_h(k_u32, s);
        
        // L = K + S + H (intermediate symbols)
        let _l = k_u32 + s + h;  // Used for documentation purposes
        
        // For Raptor10, all K source symbols are active (no inactive symbols in the precode matrix)
        // So a = k
        let params = CodeParams::new(
            k,              // k: number of source symbols
            k,              // a: all source symbols are active
            s as usize,     // l: LDPC symbols (S)
            h as usize      // h: HDPC symbols (H)
        );
        
        Self {
            params,
            k,
        }
    }

    /// Create a new Raptor10 Systematic Code with default settings
    /// 
    /// Uses HDPC10 and ReversedLDPC types
    /// 
    /// # Arguments
    /// 
    /// * `k` - Number of source symbols
    /// 
    /// # Returns
    /// 
    /// A new `Raptor10SysCode` instance with default settings
   pub fn new_with_default_setting(k: usize) -> Self {
        Self::new(k, 30.min(k))
    }

    fn dynamic_inactivation_budget(&self) -> usize {
        self.params.num_pre_inactive() + self.k / 2 + 10
    }
}

/// Calculate X value for RFC 5053: smallest X where X*(X-1) >= 2*K
fn calculate_x_val(k: u32) -> u32 {
    let mut x = 1;
    while x * (x - 1) < 2 * k {
        x += 1;
    }
    x
}

/// Calculate S for RFC 5053: smallest prime >= ceil(0.01*K) + X
fn calculate_s(k: u32, x: u32) -> u32 {
    let threshold = ((k as f64 * 0.01).ceil() as u32) + x;
    next_prime(threshold)
}

/// Calculate H for RFC 5053: smallest H where C(H, ceil(H/2)) >= K + S
fn calculate_h(k: u32, s: u32) -> u32 {
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

/// Find the smallest prime number >= n
fn next_prime(n: u32) -> u32 {
    let mut candidate = n;
    while !is_prime(candidate) {
        candidate += 1;
    }
    candidate
}

/// Check if a number is prime
fn is_prime(n: u32) -> bool {
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
fn binomial(n: u32, k: u32) -> u64 {
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

impl CodeScheme for Raptor10SysCode {
    /// Get the code parameters
    fn get_params(&self) -> CodeParams {
        self.params.clone()
    }

    /// Get the code type (Systematic)
    fn code_type(&self) -> CodeType {
        CodeType::Systematic
    }

    /// Create a degree set function using RFC 5053 degree set generator
    /// 
    /// This uses the RFC 5053 LT Encoding Symbol Generator (LTEnc[])
    /// which deterministically generates indices for intermediate symbols
    /// to be XORed together for each encoding symbol.
    fn create_degree_set_fn(&self) -> Box<dyn FnMut(usize) -> Vec<usize>> {
        // Use RFC 5053 degree set
        let mut degree_set = RFC5053DegreeSet::new(self.k);
        let threshold = self.params.num_active();
        Box::new(move |coded_id| {
            let (mut active_indices, inactive_indices) = degree_set.degree_set(coded_id);
            active_indices.extend(inactive_indices.iter().map(|&i| i + threshold));
            active_indices
        })
    }

    /// Create precode instances (both HDPC and LDPC)
    /// 
    /// Uses HDPC10 from fountain_basic and configurable LDPC type
    fn create_precode(&self) -> (Option<Box<dyn HDPC>>, Option<Box<dyn LDPC>>) {
        // Create HDPC10 from fountain_basic
        let hdpc: Option<Box<dyn HDPC>> = Some(Box::new(R10HDPC::new()));
        
        let ldpc: Option<Box<dyn LDPC>> = Some(Box::new(R10LDPC::new(&self.params)));


        (hdpc, ldpc)
    }

    fn decoding_config(&self) -> DecodingConfig {
        let mut config = DecodingConfig::default();
        if self.params.k > 500 {
            config.subs_method = SubstitutionMethod::Original;
        }
        config.with_max_inact_num(self.dynamic_inactivation_budget())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raptor10_sys_creation() {
        let k = 50;
        let dmax = 30;
        let code = Raptor10SysCode::new(k, dmax);
        
        assert_eq!(code.get_params().k, k); // k should equal input k
        assert!(code.get_params().h > 0); // Should have HDPC symbols
        assert!(code.get_params().l > 0); // Should have LDPC symbols
        assert_eq!(code.code_type(), CodeType::Systematic);
    }

    #[test]
    fn test_raptor10_sys_with_default_setting() {
        let k = 100;
        let code = Raptor10SysCode::new_with_default_setting(k);
        
        assert_eq!(code.get_params().k, k);
        assert!(code.get_params().h > 0);
        assert!(code.get_params().l > 0);
        assert_eq!(code.code_type(), CodeType::Systematic);
    }

    #[test]
    fn test_degree_set_function() {
        let k = 20;
        let code = Raptor10SysCode::new_with_default_setting(k);
        let mut degree_set_fn = code.create_degree_set_fn();
        
        // Test that the degree set function works
        let sources = degree_set_fn(0);
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_precode_creation() {
        let k = 25;
        let dmax = 15;
        let code = Raptor10SysCode::new(k, dmax);
        let (hdpc, ldpc) = code.create_precode();
        
        assert!(hdpc.is_some());
        assert!(ldpc.is_some());
    }
    
    #[test]
    fn test_rfc5053_parameters() {
        // Test that RFC 5053 parameter calculations work correctly
        let k = 100;
        let code = Raptor10SysCode::new_with_default_setting(k);
        let params = code.get_params();
        
        // L = K + S + H, where S = l, H = h
        let l_calculated = k + params.l + params.h;
        
        println!("K={}, S={}, H={}, L={}", k, params.l, params.h, l_calculated);
        
        // Verify parameters are reasonable
        assert!(params.l > 0, "S (LDPC symbols) should be > 0");
        assert!(params.h > 0, "H (HDPC symbols) should be > 0");
        assert!(l_calculated > k, "L should be > K");
    }
    
    #[test]
    fn test_helper_functions() {
        // Test X calculation
        assert_eq!(calculate_x_val(100), 15); // 15*14 = 210 >= 200
        assert_eq!(calculate_x_val(10), 5);   // 5*4 = 20 >= 20
        
        // Test prime checking
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(is_prime(5));
        assert!(!is_prime(4));
        
        // Test next_prime
        assert_eq!(next_prime(10), 11);
        assert_eq!(next_prime(11), 11);
        
        // Test binomial
        assert_eq!(binomial(5, 2), 10);
        assert_eq!(binomial(6, 3), 20);
    }
}


