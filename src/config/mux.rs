use std::{env, fs, io};
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct MuxConfig {
    pub token_id: String,
    pub token_secret: String,
    pub signing_key_id: String,
    pub private_key: String,
    pub base_url: String,
}

impl MuxConfig {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Get environment variables
        let token_id = env::var("MUX_TOKEN_ID")
            .expect("MUX_TOKEN_ID must be set");
        let token_secret = env::var("MUX_TOKEN_SECRET")
            .expect("MUX_TOKEN_SECRET must be set");
        let signing_key_id = env::var("MUX_SIGNING_KEY_ID")
            .expect("MUX_SIGNING_KEY_ID must be set");
        let base_url = "https://api.mux.com".to_string();

        // Read the PEM file from the project root
        let private_key = read_pem_key("mux.pem")?;

        Ok(Self {
            token_id,
            token_secret,
            signing_key_id,
            private_key,
            base_url,
        })
    }
}

pub fn read_pem_key<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}