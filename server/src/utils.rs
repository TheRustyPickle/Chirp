use rand::rngs::OsRng;
use rand::RngCore;
use std::fmt::Write;

/// Generate a random hex string to be used as a token
pub fn generate_user_token() -> String {
    let mut random_bytes = [0u8; 32];

    OsRng.fill_bytes(&mut random_bytes);
    let hex_string = random_bytes
        .iter()
        .fold(String::with_capacity(64), |mut acc, byte| {
            write!(&mut acc, "{:02X}", byte).expect("Failed to write to string");
            acc
        });

    hex_string
}

/// Create a message group name from 2 IDs. The smaller ID is always the first value
pub fn create_message_group(id_1: usize, id_2: usize) -> String {
    if id_1 > id_2 {
        format!("{}@{}", id_2, id_1)
    } else {
        format!("{}@{}", id_1, id_2)
    }
}
