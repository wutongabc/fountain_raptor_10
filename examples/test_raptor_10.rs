//! Test k values from 5 to 1000 with Raptor10SysCode (RFC 5053)
//! 
//! This script tests whether systematic encoding/decoding works for all k values
//! in the range [5, 1000] using Raptor10 code.
//!
//! Run with: $env:RUSTFLAGS="-A warnings" ; $env:RUST_BACKTRACE=1 ; cargo run --bin test_raptor_10 --release

use fountain_engine::*;
use fountain_utility::VecDataOperater;
use fountain_raptor_10::Raptor10SysCode;
use std::panic;

/// Test a single k value with systematic encoding/decoding
/// Test if a single k value can be successfully encoded and decoded using systematic code
fn test_single_k(k: usize) -> bool {
    let symbol_size = 4; // Size of each symbol in bytes
    
    // Use catch_unwind to capture potential panics, preventing a single test failure from crashing the entire program
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        // Attempt to create configuration, might panic if k is not supported
        let config = Raptor10SysCode::new_with_default_setting(k);
        let params = config.get_params();
        let k_prime = params.k;

        println!("params: {:?}", params);

        // Create message vectors
        let mut message_vectors = vec![vec![0u8; symbol_size]; k_prime];
        for i in 0..k_prime {
            for j in 0..symbol_size {
                message_vectors[i][j] = ((i * j + i) % 256) as u8;
            }
        }
        
        // Setup encoder with operator
        let mut encode_data_operater = VecDataOperater::new(symbol_size);
        for (i, vector) in message_vectors.iter().enumerate() {
            encode_data_operater.insert_vector(vector, i);
        }
        
        let mut encoder = Encoder::new_with_operator(&config, Box::new(encode_data_operater));
        let mut coded_id_to_data_id = std::collections::HashMap::new();
        
        // Encode systematic symbols (0..k_prime)
        for coded_id in 0..k_prime {
            if let Some(data_id) = encoder.encode_coded_vector(coded_id) {
                coded_id_to_data_id.insert(coded_id, data_id);
            }
        }
        
        // Encode extra repair symbols
        let total_num = params.num_total();
        let num_repair = k_prime / 2; // 50% overhead
        for coded_id in total_num..total_num + num_repair {
            if let Some(data_id) = encoder.encode_coded_vector(coded_id) {
                coded_id_to_data_id.insert(coded_id, data_id);
            }
        }
        
        // Get the operator back to access encoded data
        let encoder_operator = encoder.manager.move_operator();
        
        // Return necessary data for decoding check
        (config, encoder_operator, message_vectors, total_num, num_repair, k_prime, coded_id_to_data_id)
    }));

    // Check if encoding setup succeeded
    let (config, encoder_operator, message_vectors, total_num, num_repair, k_prime, coded_id_to_data_id) = match result {
        Ok(val) => val,
        Err(_) => return false, // Setup failed
    };
    
    // Now test decoding
    let mut decoder = Decoder::new_with_operator(&config, Box::new(VecDataOperater::new(symbol_size)));
    
    // Try decoding with source symbols
    let mut decoded = false;
    for coded_id in 0..k_prime {
        if let Some(data_id) = coded_id_to_data_id.get(&coded_id) {
            let status = decoder.add_coded_vector(coded_id, encoder_operator.get_vector(*data_id));
            
            if let DecodeStatus::Decoded = status {
                decoded = true;
                break;
            }
        }
    }
    
    // If not decoded, add repair symbols
    if !decoded {
        for coded_id in total_num..total_num + num_repair {
            if let Some(data_id) = coded_id_to_data_id.get(&coded_id) {
                let status = decoder.add_coded_vector(coded_id, encoder_operator.get_vector(*data_id));
                
                if let DecodeStatus::Decoded = status {
                    decoded = true;
                    break;
                }
            }
        }
    }
    
    // Verify data
    if decoded {
        let decoder_operator = decoder.manager.move_operator();
        for i in 0..k_prime {
            if decoder_operator.get_vector(i) != message_vectors[i] {
                return false; // Data mismatch
            }
        }
        return true;
    }
    
    false
}

fn main() {
    println!("=== Testing K Values from 5 to 1000 with Raptor10SysCode ===\n");
    
    // Rust Concept: Range `5..=1000` represents the interval from 5 to 1000 (inclusive).
    // let range = 5..=1000;
    let range = 4..=2000;
    
    let mut solvable_k = Vec::new();
    let mut unsolvable_k = Vec::new();
    
    println!("Testing progress:");
    println!("{}", "-".repeat(70));
    
    for k in range {
        // Print progress occasionally
        if k % 50 == 0 {
            print!("Testing k={}... ", k);
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }

        let is_solvable = test_single_k(k);
        
        if is_solvable {
            solvable_k.push(k);
            if k % 50 == 0 { println!("OK"); }
        } else {
            unsolvable_k.push(k);
            if k % 50 == 0 { println!("FAIL"); }
        }
    }
    
    println!("{}", "-".repeat(70));
    println!("\n=== Test Results Summary ===");
    println!("Total k values tested: {}", 1000 - 5 + 1);
    println!("Solvable: {}", solvable_k.len());
    println!("Unsolvable: {}", unsolvable_k.len());
    
    if !unsolvable_k.is_empty() {
        println!("\n=== Unsolvable K Values ===");
        for (i, &k) in unsolvable_k.iter().enumerate() {
            if i > 0 && i % 10 == 0 { println!(); }
            print!("{:5}, ", k);
        }
        println!();
    } else {
        println!("\nAll tested K values are solvable!");
    }
}
