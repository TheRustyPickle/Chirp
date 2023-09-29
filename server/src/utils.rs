use rand::rngs::OsRng;
use rand::RngCore;

pub fn generate_user_token() -> String {
    let mut os_rng = OsRng::default();
    let mut random_bytes = [0u8; 32];

    os_rng.fill_bytes(&mut random_bytes);
    let hex_string: String = random_bytes
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect();

    hex_string
}
