use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Course {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Video {
    pub id: Uuid,
    pub course_id: Uuid,
    pub upload_id: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub mux_asset_id: Option<String>,
    pub mux_playback_id: Option<String>,
    pub duration: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Enrollment {
    pub id: Uuid,
    pub user_id: Uuid,
    pub course_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoPlaybackRequest {
    pub video_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoPlaybackResponse {
    pub playback_id: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateVideoRequest {
    pub course_id: Uuid,
    pub title: String,
    pub upload_id: String,
}