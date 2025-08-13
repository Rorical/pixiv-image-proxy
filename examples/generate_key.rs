fn main() {
    use aes_gcm::aead::OsRng;
    use rand::RngCore;
    use base64::{Engine as _, engine::general_purpose};
    
    // Generate a 32-byte key
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    let encoded_key = general_purpose::STANDARD.encode(key);
    
    println!("Generated encryption key: {}", encoded_key);
    println!("Set this as S3_ENCRYPTION_KEY environment variable");
}