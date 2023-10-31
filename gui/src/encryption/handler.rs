use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::Aead;
use aes_gcm::{AeadCore, Aes256Gcm, KeyInit};
use gio::glib::Sender;
use pkcs1::{
    DecodeRsaPrivateKey, DecodeRsaPublicKey, EncodeRsaPrivateKey, EncodeRsaPublicKey, Error,
    LineEnding,
};
use rand::rngs::OsRng;
use rand::{thread_rng, RngCore};
use rayon::prelude::*;
use rsa::{pkcs1, Oaep, RsaPrivateKey, RsaPublicKey};
use std::path::Path;
use std::time::Duration;
use tracing::info;

use crate::ws::{DecryptedMessageData, MessageData};

pub fn generate_new_rsa_keys() -> (RsaPublicKey, RsaPrivateKey) {
    info!("Generating new RSA keys");
    let private_key =
        RsaPrivateKey::new(&mut thread_rng(), 2048).expect("failed to generate a key");
    let public_key = RsaPublicKey::from(&private_key);

    (public_key, private_key)
}

pub fn generate_new_aes_key() -> Vec<u8> {
    info!("Generating a new AES key");
    let mut aes_key = [0u8; 32];
    OsRng.fill_bytes(&mut aes_key);
    aes_key.to_vec()
}

pub fn read_rsa_keys_from_file(location: String) -> Result<(RsaPublicKey, RsaPrivateKey), Error> {
    let public_location = format!("{}public_key.pem", location);
    let private_location = format!("{}private_key.pem", location);

    let public_path = Path::new(&public_location);
    let private_path = Path::new(&private_location);

    let public_key = DecodeRsaPublicKey::read_pkcs1_pem_file(public_path)?;
    let private_key = DecodeRsaPrivateKey::read_pkcs1_pem_file(private_path)?;
    Ok((public_key, private_key))
}

pub fn read_rsa_public_from_string(key_string: String) -> RsaPublicKey {
    DecodeRsaPublicKey::from_pkcs1_pem(&key_string).unwrap()
}

pub fn stringify_rsa_keys(
    public_key: &RsaPublicKey,
    private_key: &RsaPrivateKey,
) -> (String, String) {
    let public_string = public_key.to_pkcs1_pem(LineEnding::default()).unwrap();
    let private_string = private_key
        .to_pkcs1_pem(LineEnding::default())
        .unwrap()
        .to_string();
    (public_string, private_string)
}

pub fn stringify_rsa_public(public_key: &RsaPublicKey) -> String {
    public_key.to_pkcs1_pem(LineEnding::default()).unwrap()
}

pub fn encrypt_message(
    aes_key: Vec<u8>,
    rsa_public: &RsaPublicKey,
    to_encrypt: &str,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut rng = rand::thread_rng();
    let cipher = Aes256Gcm::new(aes_key.as_slice().into());
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let encrypted_message = cipher.encrypt(&nonce, to_encrypt.as_bytes()).unwrap();
    let padding = Oaep::new::<sha2::Sha256>();
    let encrypted_aes_key = rsa_public
        .encrypt(&mut rng, padding, aes_key.as_slice())
        .unwrap();

    (encrypted_message, encrypted_aes_key, nonce.to_vec())
}

pub fn decrypt_message(
    message_data: MessageData,
    rsa_private_key: &RsaPrivateKey,
    owner_id: u64,
) -> DecryptedMessageData {
    let is_send = owner_id == message_data.from_user;

    let (text_data, aes_key, nonce) = if is_send {
        (
            message_data.sender_message.unwrap(),
            message_data.sender_key.unwrap(),
            message_data.sender_nonce.unwrap(),
        )
    } else {
        (
            message_data.receiver_message.unwrap(),
            message_data.receiver_key.unwrap(),
            message_data.receiver_nonce.unwrap(),
        )
    };
    let nonce = GenericArray::from_slice(nonce.as_slice());

    let padding = Oaep::new::<sha2::Sha256>();

    let aes_key = rsa_private_key.decrypt(padding, &aes_key).unwrap();

    let cipher = Aes256Gcm::new(aes_key.as_slice().into());
    let message_bytes = cipher.decrypt(nonce, text_data.as_ref()).unwrap();

    let message_text = String::from_utf8(message_bytes).unwrap();

    DecryptedMessageData::new(
        message_data.created_at,
        message_data.from_user,
        message_data.to_user,
        message_text,
        message_data.message_number,
    )
}

pub fn decrypt_message_chunk(
    sender: Sender<(Vec<DecryptedMessageData>, bool)>,
    message_data: Vec<MessageData>,
    rsa_private_key: &RsaPrivateKey,
    owner_id: u64,
) {
    let chunk_data = message_data.chunks(10);
    let chunk_len = chunk_data.len() - 1;

    for (index, chunk) in chunk_data.enumerate() {
        let decrypted_chunk: Vec<DecryptedMessageData> = chunk
            .to_vec()
            .into_par_iter()
            .map(|message| {
                if message.sender_key.is_none() {
                    return DecryptedMessageData::new_empty_message(
                        message.created_at,
                        message.from_user,
                        message.to_user,
                        message.message_number,
                    );
                }

                let decrypted_message = decrypt_message(message, rsa_private_key, owner_id);

                decrypted_message
            })
            .collect();
        std::thread::sleep(Duration::from_secs(1));
        let completed = index == chunk_len;
        sender.send((decrypted_chunk, completed)).unwrap();
    }
}
