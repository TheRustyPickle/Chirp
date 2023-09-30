use rand::rngs::OsRng;
use rand::RngCore;

pub fn generate_user_token() -> String {
    let mut random_bytes = [0u8; 32];

    OsRng.fill_bytes(&mut random_bytes);
    let hex_string: String = random_bytes
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect();

    hex_string
}
