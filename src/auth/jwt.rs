use chrono::{Duration, Utc};
use jsonwebtoken::{encode, decode, DecodingKey, EncodingKey, Header, Validation, errors::Error as JwtError};
use serde::{Serialize, Deserialize};

// claims structure that will be encoded in the JWT
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
}

const JWT_SECRET: &[u8] = b"my-secret-key";
const TOKEN_EXPIRATION_TIME: Duration = Duration::seconds(120);

pub fn generate_token(email: String) -> Result<String, JwtError> {
    let now = Utc::now();
    let expires_at = now + Duration::seconds(TOKEN_EXPIRATION_TIME.num_seconds());

    // preparing claims for the token
    let claims = Claims {
        sub: email,
        exp: expires_at.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )

}

pub fn validate_token(token: String) -> Result<Claims, JwtError> {
    decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )
        .map(|data| data.claims)
}