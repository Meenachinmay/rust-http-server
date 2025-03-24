use crate::config::mux::MuxConfig;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, AUTHORIZATION};

// Add these new structs for direct upload
#[derive(Serialize, Deserialize, Debug)]
pub struct UploadResponse {
    pub data: UploadData,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UploadData {
    pub id: String,
    pub url: String,
    pub cors_origin: String,
    pub status: String,
    #[serde(default)]
    pub timeout: i32,
    pub new_asset_settings: NewAssetSettings, // Using our unified struct
}

// A single NewAssetSettings struct for both request and response
#[derive(Serialize, Deserialize, Debug)]
pub struct NewAssetSettings {
    // This field will be "playback_policy" when sending requests
    // and "playback_policies" when receiving responses
    #[serde(rename(serialize = "playback_policy", deserialize = "playback_policies"))]
    pub playback_policy: Vec<String>,

    // Make video_quality a String instead of Option<String>
    #[serde(default)]
    pub video_quality: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateUploadRequest {
    pub cors_origin: String,
    pub new_asset_settings: NewAssetSettings,
}

#[derive(Debug, Clone)]
pub struct MuxService {
    config: MuxConfig,
    client: Client,
}

#[derive(Serialize, Deserialize)]
pub struct Video {
    pub id: String,
    pub playback_id: String,
    pub title: String,
    pub duration: f64,
}

#[derive(Serialize, Deserialize)]
pub struct VideoAssetResponse {
    pub data: VideoAsset,
}

#[derive(Serialize, Deserialize)]
pub struct VideoAsset {
    pub id: String,
    pub playback_ids: Vec<PlaybackId>,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct PlaybackId {
    pub id: String,
    pub policy: String,
}

#[derive(Serialize, Deserialize)]
struct JwtClaims {
    sub: String,
    exp: usize,
    aud: String,
    kid: String,
}

impl MuxService {
    pub fn new(config: MuxConfig) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    pub fn create_playback_token(&self, playback_id: &str) -> Result<String, Box<dyn Error>> {
        let now = Utc::now();
        let expiry = now + Duration::hours(6); // Token valid for 6 hours

        let claims = JwtClaims {
            sub: playback_id.to_string(),
            aud: "v".to_string(), // 'v' is for video
            exp: expiry.timestamp() as usize,
            kid: self.config.signing_key_id.clone(),
        };

        // Create a header specifying RS256 algorithm
        let header = Header::new(Algorithm::RS256);

        let encoding_key = EncodingKey::from_rsa_pem(self.config.private_key.as_bytes())?;

        let token = encode(
            &header,
            &claims,
            &encoding_key,
        )?;

        Ok(token)
    }

    pub async fn create_direct_upload(&self) -> Result<UploadData, Box<dyn std::error::Error>> {
        // Create the request body
        let request_body = CreateUploadRequest {
            cors_origin: "*".to_string(),
            new_asset_settings: NewAssetSettings {
                playback_policy: vec!["public".to_string()],
                video_quality: "basic".to_string(),
            },
        };

        // Set up authentication for Mux API
        let auth = format!("{}:{}", self.config.token_id, self.config.token_secret);
        let encoded_auth = base64::encode(auth);

        // Set up headers
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Basic {}", encoded_auth))
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
        );

        // Make the request to Mux API
        let response = self.client
            .post(&format!("{}/video/v1/uploads", self.config.base_url))
            .headers(headers)
            .json(&request_body)
            .send()
            .await?;

        // Check if the request was successful
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            return Err(format!("Failed to create upload. Status: {}, Response: {}", status, text).into());
        }

        // Parse the response
        let upload_response: UploadResponse = response.json().await?;
        Ok(upload_response.data)
    }

    // need to update this method
    pub async fn list_videos(&self) -> Result<Vec<Video>, Box<dyn Error>> {
        let url = format!("{}/video/v1/assets", self.config.base_url);

        let response = self.client
            .get(&url)
            .basic_auth(&self.config.token_id, Some(&self.config.token_secret))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        // Process the response to extract video details
        let videos = response["data"].as_array()
            .ok_or("Invalid response format")?
            .iter()
            .filter_map(|asset| {
                let id = asset["id"].as_str()?;
                let playback_id = asset["playback_ids"].as_array()?
                    .first()?["id"].as_str()?;
                let title = asset["metadata"]["title"].as_str().unwrap_or("Untitled");
                let duration = asset["duration"].as_f64().unwrap_or(0.0);

                Some(Video {
                    id: id.to_string(),
                    playback_id: playback_id.to_string(),
                    title: title.to_string(),
                    duration,
                })
            })
            .collect();

        Ok(videos)
    }

    #[cfg(test)]
    pub fn with_mock_list_videos() -> Self {
        let config = MuxConfig {
            token_id: "test_token_id".to_string(),
            token_secret: "test_token_secret".to_string(),
            signing_key_id: "test_signing_key_id".to_string(),
            private_key: String::from("-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----"),
            base_url: "https://api.mux.com".to_string(),
        };

        Self { config, client: Client::new() }
    }

    #[cfg(test)]
    pub async fn list_videos_test(&self) -> Result<Vec<Video>, Box<dyn Error>> {
        // Return mock data instead of making an actual API call
        Ok(vec![
            Video {
                id: "mock_asset_id_1".to_string(),
                playback_id: "mock_playback_id_1".to_string(),
                title: "Mock Video 1".to_string(),
                duration: 120.0,
            },
            Video {
                id: "mock_asset_id_2".to_string(),
                playback_id: "mock_playback_id_2".to_string(),
                title: "Mock Video 2".to_string(),
                duration: 180.0,
            },
        ])
    }
}

#[cfg(test)]
mod mux_service_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_create_playback_token() {
        // Create a service with the test configuration
        let private_key = fs::read_to_string("mux.pem")
            .expect("Failed to read mux.pem file");

        let config = MuxConfig {
            token_id: "test_token_id".to_string(),
            token_secret: "test_token_secret".to_string(),
            signing_key_id: "test_signing_key_id".to_string(), // Add this
            private_key,
            base_url: "https://api.mux.com".to_string(),
        };

        let service = MuxService::new(config);
        let playback_id = "test_playback_id";

        let result = service.create_playback_token(playback_id);

        assert!(result.is_ok(), "Token creation should succeed");
        let token = result.unwrap();
        assert!(!token.is_empty(), "Token should not be empty");

        // Verify token structure
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT should have three parts");

        // Decode the base64 payload to verify claims
        let payload = base64::decode_config(parts[1], base64::URL_SAFE_NO_PAD)
            .expect("Failed to decode base64 payload");
        let payload_str = String::from_utf8_lossy(&payload);

        let claims: serde_json::Value = serde_json::from_str(&payload_str)
            .expect("Failed to parse claims JSON");

        // Validate the claims match what we expect
        assert_eq!(claims["sub"].as_str().unwrap(), playback_id,
                   "Token subject should be the playback ID");
        assert_eq!(claims["aud"].as_str().unwrap(), "v",
                   "Token audience should be 'v'");
        assert_eq!(claims["kid"].as_str().unwrap(), "test_signing_key_id",
                   "Token kid should be signing key ID");

        // Verify the algorithm in the header
        let header = base64::decode_config(parts[0], base64::URL_SAFE_NO_PAD)
            .expect("Failed to decode base64 header");
        let header_str = String::from_utf8_lossy(&header);
        let header_json: serde_json::Value = serde_json::from_str(&header_str)
            .expect("Failed to parse header JSON");

        assert_eq!(header_json["alg"].as_str().unwrap(), "RS256",
                   "Token algorithm should be RS256");
    }
}