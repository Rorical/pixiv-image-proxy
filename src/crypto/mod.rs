use anyhow::{Result, anyhow};
use bytes::Bytes;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use std::io::{Read, Write};
use base64::{Engine as _, engine::general_purpose};
use rand::RngCore;

use crate::config::{EncryptionConfig, CompressionConfig};

#[derive(Clone)]
pub struct CryptoProcessor {
    encryption_config: EncryptionConfig,
    compression_config: CompressionConfig,
    encryption_key: Option<Key<Aes256Gcm>>,
}

impl CryptoProcessor {
    pub fn new(encryption_config: EncryptionConfig, compression_config: CompressionConfig) -> Result<Self> {
        let encryption_key = if encryption_config.enabled {
            let key_bytes = if let Some(key_str) = &encryption_config.key {
                // Decode base64 key
                general_purpose::STANDARD.decode(key_str)
                    .map_err(|e| anyhow!("Failed to decode encryption key: {}", e))?
            } else {
                return Err(anyhow!("Encryption is enabled but no key provided"));
            };

            if key_bytes.len() != 32 {
                return Err(anyhow!("Encryption key must be 32 bytes (256 bits)"));
            }

            let mut key_array = [0u8; 32];
            key_array.copy_from_slice(&key_bytes);
            Some(Key::<Aes256Gcm>::from_slice(&key_array).clone())
        } else {
            None
        };

        Ok(Self {
            encryption_config,
            compression_config,
            encryption_key,
        })
    }

    pub async fn process_for_storage(&self, data: Bytes) -> Result<Bytes> {
        let mut processed_data = data;

        // Apply compression first if enabled
        if self.compression_config.enabled {
            processed_data = self.compress(processed_data)?;
        }

        // Apply encryption if enabled
        if self.encryption_config.enabled {
            processed_data = self.encrypt(processed_data)?;
        }

        Ok(processed_data)
    }

    pub async fn process_for_retrieval(&self, data: Bytes) -> Result<Bytes> {
        let mut processed_data = data;

        // Reverse the order: decrypt first, then decompress
        if self.encryption_config.enabled {
            processed_data = self.decrypt(processed_data)?;
        }

        if self.compression_config.enabled {
            processed_data = self.decompress(processed_data)?;
        }

        Ok(processed_data)
    }

    fn compress(&self, data: Bytes) -> Result<Bytes> {
        match self.compression_config.algorithm.as_str() {
            "gzip" => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.compression_config.level));
                encoder.write_all(&data)
                    .map_err(|e| anyhow!("Failed to compress data: {}", e))?;
                let compressed = encoder.finish()
                    .map_err(|e| anyhow!("Failed to finish compression: {}", e))?;
                Ok(Bytes::from(compressed))
            },
            _ => Err(anyhow!("Unsupported compression algorithm: {}", self.compression_config.algorithm)),
        }
    }

    fn decompress(&self, data: Bytes) -> Result<Bytes> {
        match self.compression_config.algorithm.as_str() {
            "gzip" => {
                let mut decoder = GzDecoder::new(&data[..]);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed)
                    .map_err(|e| anyhow!("Failed to decompress data: {}", e))?;
                Ok(Bytes::from(decompressed))
            },
            _ => Err(anyhow!("Unsupported compression algorithm: {}", self.compression_config.algorithm)),
        }
    }

    fn encrypt(&self, data: Bytes) -> Result<Bytes> {
        match self.encryption_config.algorithm.as_str() {
            "AES-256-GCM" => {
                let key = self.encryption_key.as_ref()
                    .ok_or_else(|| anyhow!("Encryption key not available"))?;
                
                let cipher = Aes256Gcm::new(key);
                
                // Generate random nonce
                let mut nonce_bytes = [0u8; 12];
                OsRng.fill_bytes(&mut nonce_bytes);
                let nonce = Nonce::from_slice(&nonce_bytes);
                
                // Encrypt the data
                let ciphertext = cipher.encrypt(nonce, data.as_ref())
                    .map_err(|e| anyhow!("Encryption failed: {}", e))?;
                
                // Prepend nonce to ciphertext
                let mut result = Vec::with_capacity(12 + ciphertext.len());
                result.extend_from_slice(&nonce_bytes);
                result.extend_from_slice(&ciphertext);
                
                Ok(Bytes::from(result))
            },
            _ => Err(anyhow!("Unsupported encryption algorithm: {}", self.encryption_config.algorithm)),
        }
    }

    fn decrypt(&self, data: Bytes) -> Result<Bytes> {
        match self.encryption_config.algorithm.as_str() {
            "AES-256-GCM" => {
                if data.len() < 12 {
                    return Err(anyhow!("Encrypted data too short"));
                }
                
                let key = self.encryption_key.as_ref()
                    .ok_or_else(|| anyhow!("Encryption key not available"))?;
                
                let cipher = Aes256Gcm::new(key);
                
                // Extract nonce and ciphertext
                let (nonce_bytes, ciphertext) = data.split_at(12);
                let nonce = Nonce::from_slice(nonce_bytes);
                
                // Decrypt the data
                let plaintext = cipher.decrypt(nonce, ciphertext)
                    .map_err(|e| anyhow!("Decryption failed: {}", e))?;
                
                Ok(Bytes::from(plaintext))
            },
            _ => Err(anyhow!("Unsupported encryption algorithm: {}", self.encryption_config.algorithm)),
        }
    }
}

pub fn generate_encryption_key() -> String {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    general_purpose::STANDARD.encode(key)
}