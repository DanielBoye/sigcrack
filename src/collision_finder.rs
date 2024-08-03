use anyhow::{Context, Result};
use tiny_keccak::{Hasher, Keccak};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use crate::output_handler::{start_output_thread, print_found_signature};

// define character set for function names
const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

pub struct CollisionFinder;

impl CollisionFinder {
    pub fn new() -> Self {
        Self
    }

    // main function to find matching signature
    pub fn find_matching_signature(
        &self,
        target_hash: &[u8],
        prefix: &str,
        length: usize,
        suffix: Option<&str>,
    ) -> Result<()> {
        let target_hash = target_hash.to_vec();
        let guess_counter = Arc::new(AtomicUsize::new(0));
        let latest_function_guess = Arc::new(Mutex::new(String::new()));
        let start_time = Instant::now();

        // start output thread for progress reporting
        start_output_thread(Arc::clone(&guess_counter), Arc::clone(&latest_function_guess), start_time);

        // choose appropriate search method based on input
        if !prefix.is_empty() && suffix.is_some() {
            self.find_with_prefix_and_suffix(&target_hash, prefix, length, suffix.unwrap(), &guess_counter, &latest_function_guess, start_time)
        } else if !prefix.is_empty() {
            self.find_with_prefix(&target_hash, prefix, length, &guess_counter, &latest_function_guess, start_time)
        } else if suffix.is_some() {
            self.find_with_suffix(&target_hash, length, suffix.unwrap(), &guess_counter, &latest_function_guess, start_time)
        } else {
            self.find_without_prefix_and_suffix(&target_hash, length, &guess_counter, &latest_function_guess, start_time)
        }
    }

    // helper functions for different search scenarios
    pub fn find_matching_signature_with_suffix(
        &self,
        target_hash: &[u8],
        length: usize,
        suffix: &str,
    ) -> Result<()> {
        self.find_matching_signature(target_hash, "", length, Some(suffix))
    }

    pub fn find_matching_signature_with_length(
        &self,
        target_hash: &[u8],
        length: usize,
    ) -> Result<()> {
        self.find_matching_signature(target_hash, "", length, None)
    }

    pub fn find_matching_signature_with_prefix_and_suffix(
        &self,
        target_hash: &[u8],
        prefix: &str,
        length: usize,
        suffix: &str,
    ) -> Result<()> {
        self.find_matching_signature(target_hash, prefix, length, Some(suffix))
    }

    // search with both prefix and suffix
    fn find_with_prefix_and_suffix(
        &self,
        target_hash: &[u8],
        prefix: &str,
        length: usize,
        suffix: &str,
        guess_counter: &Arc<AtomicUsize>,
        latest_function_guess: &Arc<Mutex<String>>,
        start_time: Instant,
    ) -> Result<()> {
        println!("Trying function names with {} characters (including prefix and suffix)", length);
        let mut buffer = vec![CHARSET[0]; length];

        loop {
            let function_name = format!("{}{}", prefix, String::from_utf8(buffer.clone())
                .with_context(|| format!("Failed to create UTF-8 string from buffer: {:?}", buffer))?);

            let candidate = format!("{}{}", function_name, suffix);

            if self.check_hash(&candidate, target_hash, guess_counter, latest_function_guess, start_time)? {
                return Ok(());
            }

            if !self.increment_buffer(&mut buffer) {
                break;
            }
        }

        Ok(())
    }

    // search with prefix only
    fn find_with_prefix(
        &self,
        target_hash: &[u8],
        prefix: &str,
        length: usize,
        guess_counter: &Arc<AtomicUsize>,
        latest_function_guess: &Arc<Mutex<String>>,
        start_time: Instant,
    ) -> Result<()> {
        println!("Trying function names with {} characters (including prefix)", length);
        let mut buffer = vec![CHARSET[0]; length];

        loop {
            let function_name = format!("{}{}", prefix, String::from_utf8(buffer.clone())
                .with_context(|| format!("Failed to create UTF-8 string from buffer: {:?}", buffer))?);

            let candidate = format!("{}()", function_name);

            if self.check_hash(&candidate, target_hash, guess_counter, latest_function_guess, start_time)? {
                return Ok(());
            }

            if !self.increment_buffer(&mut buffer) {
                break;
            }
        }

        Ok(())
    }

    // search with suffix only
    fn find_with_suffix(
        &self,
        target_hash: &[u8],
        length: usize,
        suffix: &str,
        guess_counter: &Arc<AtomicUsize>,
        latest_function_guess: &Arc<Mutex<String>>,
        start_time: Instant,
    ) -> Result<()> {
        println!("Trying function names with {} characters (including suffix)", length);
        let mut buffer = vec![CHARSET[0]; length];

        loop {
            let function_name = String::from_utf8(buffer.clone())
                .with_context(|| format!("Failed to create UTF-8 string from buffer: {:?}", buffer))?;

            let candidate = format!("{}{}", function_name, suffix);

            if self.check_hash(&candidate, target_hash, guess_counter, latest_function_guess, start_time)? {
                return Ok(());
            }

            if !self.increment_buffer(&mut buffer) {
                break;
            }
        }

        Ok(())
    }

    // search without prefix or suffix
    fn find_without_prefix_and_suffix(
        &self,
        target_hash: &[u8],
        length: usize,
        guess_counter: &Arc<AtomicUsize>,
        latest_function_guess: &Arc<Mutex<String>>,
        start_time: Instant,
    ) -> Result<()> {
        println!("Trying function names with {} characters", length);
        let mut buffer = vec![CHARSET[0]; length];

        loop {
            let function_name = String::from_utf8(buffer.clone())
                .with_context(|| format!("Failed to create UTF-8 string from buffer: {:?}", buffer))?;

            let candidate = format!("{}()", function_name);

            if self.check_hash(&candidate, target_hash, guess_counter, latest_function_guess, start_time)? {
                return Ok(());
            }

            if !self.increment_buffer(&mut buffer) {
                break;
            }
        }

        Ok(())
    }

    // check if the hash matches the target
    fn check_hash(
        &self,
        function_name: &str,
        target_hash: &[u8],
        guess_counter: &Arc<AtomicUsize>,
        latest_function_guess: &Arc<Mutex<String>>,
        start_time: Instant,
    ) -> Result<bool> {
        let hash = self.keccak256(function_name);
        let hash_prefix = &hash[..4];

        guess_counter.fetch_add(1, Ordering::Relaxed);
        *latest_function_guess.lock().unwrap() = function_name.to_string();

        if hash_prefix == target_hash {
            print_found_signature(function_name, start_time, guess_counter);
            return Ok(true);
        }

        Ok(false)
    }

    // increment the buffer to generate the next function name
    fn increment_buffer(&self, buffer: &mut Vec<u8>) -> bool {
        for i in (0..buffer.len()).rev() {
            let current_char_index = CHARSET.iter().position(|&c| c == buffer[i]).unwrap();
            if current_char_index < CHARSET.len() - 1 {
                buffer[i] = CHARSET[current_char_index + 1];
                return true;
            } else {
                buffer[i] = CHARSET[0];
            }
        }
        false
    }

    // compute keccak256 hash of the input string
    fn keccak256(&self, input: &str) -> Vec<u8> {
        let mut hasher = Keccak::v256();
        let mut output = [0u8; 32];
        hasher.update(input.as_bytes());
        hasher.finalize(&mut output);
        output.to_vec()
    }
}