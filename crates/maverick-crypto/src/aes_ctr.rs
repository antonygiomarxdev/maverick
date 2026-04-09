use aes::Aes128;
use cmac::Mac;
type CmacEngine = cmac::Cmac<Aes128>;

use crate::error::{CryptoError, Result};
use cipher::{KeyInit, KeyIvInit, StreamCipher};

const AES_KEY_SIZE: usize = 16;

#[derive(Debug, Clone)]
pub struct AesCtr {
    key: [u8; AES_KEY_SIZE],
}

impl AesCtr {
    pub fn new(key: &[u8]) -> Result<Self> {
        if key.len() != AES_KEY_SIZE {
            return Err(CryptoError::invalid_key_length(AES_KEY_SIZE, key.len()));
        }
        let mut key_arr = [0u8; AES_KEY_SIZE];
        key_arr.copy_from_slice(key);
        Ok(Self { key: key_arr })
    }

    pub fn process(&self, iv: &[u8], input: &[u8], output: &mut [u8]) -> Result<()> {
        if iv.len() != 16 {
            return Err(CryptoError::invalid_iv_length(16, iv.len()));
        }
        if output.len() < input.len() {
            return Err(CryptoError::BufferOverflow {
                requested: input.len(),
                available: output.len(),
            });
        }

        output[..input.len()].copy_from_slice(input);

        let mut cipher = ctr::Ctr128BE::<Aes128>::new(&self.key.into(), iv.into());

        cipher.apply_keystream(&mut output[..input.len()]);
        Ok(())
    }

    pub fn process_slice(&self, iv: &[u8], input: &[u8]) -> Result<Vec<u8>> {
        let mut output = vec![0u8; input.len()];
        self.process(iv, input, &mut output)?;
        Ok(output)
    }

    pub fn decrypt_slice(&self, iv: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
        self.process_slice(iv, ciphertext)
    }
}

#[derive(Debug, Clone)]
pub struct Cmac {
    key: [u8; AES_KEY_SIZE],
}

impl Cmac {
    pub fn new(key: &[u8]) -> Result<Self> {
        if key.len() != AES_KEY_SIZE {
            return Err(CryptoError::invalid_key_length(AES_KEY_SIZE, key.len()));
        }
        let mut key_arr = [0u8; AES_KEY_SIZE];
        key_arr.copy_from_slice(key);
        Ok(Self { key: key_arr })
    }

    pub fn compute(&self, data: &[u8]) -> Result<[u8; 16]> {
        let mut mac = <CmacEngine as KeyInit>::new_from_slice(&self.key)
            .map_err(|_| CryptoError::InternalError("AES-CMAC key init failed"))?;
        mac.update(data);
        let result = mac.finalize().into_bytes();
        let mut out = [0u8; 16];
        out.copy_from_slice(&result);
        Ok(out)
    }

    pub fn verify(&self, data: &[u8], expected_mic: &[u8]) -> Result<bool> {
        if expected_mic.len() != 4 {
            return Err(CryptoError::InvalidMicSize {
                expected: 4,
                got: expected_mic.len(),
            });
        }
        let computed = self.compute(data)?;
        let computed_mic: [u8; 4] = [computed[0], computed[1], computed[2], computed[3]];
        Ok(constant_time_compare(&computed_mic, expected_mic))
    }
}

fn constant_time_compare(a: &[u8; 4], b: &[u8]) -> bool {
    if b.len() != 4 {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..4 {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_ctr_new_invalid_key() {
        assert!(AesCtr::new(&[0u8; 15]).is_err());
    }

    #[test]
    fn test_aes_ctr_new_valid_key() {
        assert!(AesCtr::new(&[0u8; 16]).is_ok());
    }

    #[test]
    fn test_cmac_new_invalid_key() {
        assert!(Cmac::new(&[0u8; 15]).is_err());
    }

    #[test]
    fn test_cmac_new_valid_key() {
        assert!(Cmac::new(&[0u8; 16]).is_ok());
    }
}
