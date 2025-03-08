use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;
use url::form_urlencoded;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: u64,
}

pub fn parse(init_data: &str) -> Option<u64> {
    if init_data.is_empty() {
        return None;
    }

    if init_data.contains(';') || !init_data.contains('=') {
        return None;
    }

    let pairs = form_urlencoded::parse(init_data.as_bytes());

    for (key, value) in pairs {
        if key == "user" {
            let user_data = serde_json::from_str::<User>(&value).ok();

            return match user_data {
                Some(user) => Some(user.id),
                None => None,
            };
        }
    }

    None
}

fn extract_hash(init_data: &str) -> Option<(String, String)> {
    let (base_data, hash) = if let Some(pos) = init_data.find("&hash=") {
        let (base, hash_part) = init_data.split_at(pos);
        let hash = &hash_part[6..]; // Skip "&hash="
        (base.to_string(), hash.to_string())
    } else {
        return None;
    };

    if !hash.chars().all(|c| c.is_ascii_hexdigit()) || hash.len() != 64 {
        return None;
    }

    Some((base_data, hash))
}

fn sign(data: &str, token: &str) -> Result<String, ()> {
    let secret_key = {
        let mut mac = HmacSha256::new_from_slice(token.as_bytes()).unwrap();
        mac.update(b"WebAppData");
        mac.finalize().into_bytes()
    };

    let token_bytes = {
        let mut mac = HmacSha256::new_from_slice(data.as_bytes()).unwrap();
        mac.update(&secret_key);
        mac.finalize().into_bytes()
    };

    Ok(hex::encode(token_bytes))
}

pub fn validate(init_data: &str, token: &str) -> Option<u64> {
    if init_data.is_empty() || !init_data.contains('=') {
        return None;
    }

    let (base_data, hash) = match extract_hash(init_data) {
        Some(v) => v,
        None => return None,
    };
    let expected_hash = match sign(&base_data, token) {
        Ok(v) => v,
        Err(_) => return None,
    };

    if hash != expected_hash {
        return None;
    }

    parse(&base_data)
}
