// src/models.rs - FIXED VERSION
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
// At the top of models.rs, add these imports
use std::collections::HashMap;

// ==================== AUTHENTICATION MODELS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub wallet_address: Option<String>,
    pub wallet_type: Option<String>,
    pub wallet_chain: Option<String>,
    pub public_key: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    // Profile completion fields
    pub account_type: Option<String>,
    pub business_name: Option<String>,
    pub geo_latitude: Option<f64>,
    pub geo_longitude: Option<f64>,
    pub is_profile_complete: bool,
    // Kenya-specific fields
    pub phone_number: Option<String>,
    pub county: Option<String>,
    pub id_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionUser {
    pub id: String,
    pub email: String,
    pub has_password: bool,
    pub has_wallet: bool,
    pub wallet_address: Option<String>,
    pub wallet_type: Option<String>,
    pub wallet_chain: Option<String>,
    pub is_verified: bool,
    pub wallet_connections: Vec<WalletConnection>,
    // Profile completion fields
    pub account_type: Option<String>,
    pub business_name: Option<String>,
    pub geo_latitude: Option<f64>,
    pub geo_longitude: Option<f64>,
    pub is_profile_complete: bool,
    // Kenya-specific fields
    pub phone_number: Option<String>,
    pub county: Option<String>,
    pub id_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletConnection {
    pub wallet_address: String,
    pub chain: String,
    pub wallet_type: String,
    pub public_key: Option<String>,
    pub connected_at: DateTime<Utc>,
    pub last_used: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentSignature {
    pub content_id: String,
    pub wallet_address: String,
    pub signature: String,
    pub signed_hash: String,
    pub timestamp: DateTime<Utc>,
    pub chain: String,
    pub transaction_id: Option<String>,
}

// ==================== EVIDENCE MODELS ====================

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EmergencyLevel {
    Red,
    Orange,
    Yellow,
    Blue,
}

impl EmergencyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EmergencyLevel::Red => "red",
            EmergencyLevel::Orange => "orange",
            EmergencyLevel::Yellow => "yellow",
            EmergencyLevel::Blue => "blue",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "red" => Some(EmergencyLevel::Red),
            "orange" => Some(EmergencyLevel::Orange),
            "yellow" => Some(EmergencyLevel::Yellow),
            "blue" => Some(EmergencyLevel::Blue),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum IncidentType {
    HitAndRun,
    Assault,
    ThreatToLife,
    PropertyDamage,
    Theft,
    Other,
}

impl IncidentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IncidentType::HitAndRun => "HitAndRun",
            IncidentType::Assault => "Assault",
            IncidentType::ThreatToLife => "ThreatToLife",
            IncidentType::PropertyDamage => "PropertyDamage",
            IncidentType::Theft => "Theft",
            IncidentType::Other => "Other",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "HitAndRun" => Some(IncidentType::HitAndRun),
            "Assault" => Some(IncidentType::Assault),
            "ThreatToLife" => Some(IncidentType::ThreatToLife),
            "PropertyDamage" => Some(IncidentType::PropertyDamage),
            "Theft" => Some(IncidentType::Theft),
            "Other" => Some(IncidentType::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum VehicleType {
    Matatu,
    BodaBoda,
    Private,
    PSV,
    Lorry,
    Taxi,
    Unknown,
}

impl VehicleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            VehicleType::Matatu => "Matatu",
            VehicleType::BodaBoda => "BodaBoda",
            VehicleType::Private => "Private",
            VehicleType::PSV => "PSV",
            VehicleType::Lorry => "Lorry",
            VehicleType::Taxi => "Taxi",
            VehicleType::Unknown => "Unknown",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Matatu" => Some(VehicleType::Matatu),
            "BodaBoda" => Some(VehicleType::BodaBoda),
            "Private" => Some(VehicleType::Private),
            "PSV" => Some(VehicleType::PSV),
            "Lorry" => Some(VehicleType::Lorry),
            "Taxi" => Some(VehicleType::Taxi),
            "Unknown" => Some(VehicleType::Unknown),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum EvidenceStatus {
    Draft,
    Submitted,
    Reported,
    UnderReview,
    Archived,
    Rejected,
}

impl EvidenceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            EvidenceStatus::Draft => "draft",
            EvidenceStatus::Submitted => "submitted",
            EvidenceStatus::Reported => "reported",
            EvidenceStatus::UnderReview => "under_review",
            EvidenceStatus::Archived => "archived",
            EvidenceStatus::Rejected => "rejected",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(EvidenceStatus::Draft),
            "submitted" => Some(EvidenceStatus::Submitted),
            "reported" => Some(EvidenceStatus::Reported),
            "under_review" => Some(EvidenceStatus::UnderReview),
            "archived" => Some(EvidenceStatus::Archived),
            "rejected" => Some(EvidenceStatus::Rejected),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustodyRecord {
    pub action: String,
    pub timestamp: DateTime<Utc>,
    pub user_id: String,
    pub details: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceLocation {
    pub county: String,
    pub constituency: Option<String>,
    pub ward: Option<String>,
    pub latitude: f64,
    pub longitude: f64,
    pub landmark: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VehicleDetails {
    pub registration: Option<String>,
    pub color: Option<String>,
    pub vehicle_type: Option<VehicleType>,
    pub make_model: Option<String>,
    pub sacco_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaFile {
    pub id: String,
    pub filename: String,
    pub mime_type: String,
    pub file_size: u64,
    pub duration_seconds: Option<u32>,
    pub thumbnail_url: Option<String>,
    pub storj_url: String,
    pub storj_key: String,
    pub hash: String,
    pub quality_rating: i32,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Evidence {
    pub id: String,
    pub evidence_number: String,
    pub emergency_level: EmergencyLevel,
    pub incident_type: IncidentType,
    pub sub_type: Option<String>,
    pub incident_time: DateTime<Utc>,
    pub report_time: DateTime<Utc>,
    pub location: EvidenceLocation,
    pub vehicle_details: Option<VehicleDetails>,
    pub title: String,
    pub description: String,
    pub injuries: Option<String>,
    pub property_damage: Option<String>,
    pub suspect_description: Option<String>,
    pub uploader_id: String,
    pub uploader_email: String,
    pub uploader_phone: Option<String>,
    pub media_files: Vec<MediaFile>,
    pub evidence_quality: i32,
    pub reported_to_police: bool,
    pub police_case_id: Option<String>,
    pub police_station: Option<String>,
    pub report_date: Option<DateTime<Utc>>,
    pub wallet_signature: Option<String>,
    pub wallet_address: Option<String>,
    pub signature_timestamp: Option<DateTime<Utc>>,
    pub status: EvidenceStatus,
    pub needs_attention: bool,
    pub is_anonymous: bool,
    pub chain_of_custody: Vec<CustodyRecord>,
    pub storj_urls: Vec<String>,
    pub storj_bucket: Option<String>,
    pub file_size_bytes: u64,
    pub mime_types: Vec<String>,
    pub hash_values: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceSummary {
    pub id: String,
    pub evidence_number: String,
    pub emergency_level: EmergencyLevel,
    pub incident_type: IncidentType,
    pub title: String,
    pub county: String,
    pub incident_time: DateTime<Utc>,
    pub status: EvidenceStatus,
    pub reported_to_police: bool,
    pub police_case_id: Option<String>,
    pub has_media: bool,
    pub needs_attention: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceSignature {
    pub evidence_id: String,
    pub wallet_address: String,
    pub signature: String,
    pub signed_hash: String,
    pub timestamp: DateTime<Utc>,
    pub chain: String,
    pub transaction_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EvidenceForm {
    pub title: String,
    pub description: String,
    pub emergency_level: String,
    pub incident_type: String,
    pub sub_type: Option<String>,
    pub incident_date: String,
    pub incident_time: String,
    pub county: String,
    pub constituency: Option<String>,
    pub ward: Option<String>,
    // lat/lon are Option<f64> so empty strings from the JS geolocation
    // fallback are treated as NULL instead of 0.0
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub landmark: Option<String>,
    pub vehicle_registration: Option<String>,
    pub vehicle_color: Option<String>,
    pub vehicle_type: Option<String>,
    pub sacco_name: Option<String>,
    pub injuries: Option<String>,
    pub property_damage: Option<String>,
    pub suspect_description: Option<String>,
    pub reported_to_police: bool,
    pub police_case_id: Option<String>,
    pub police_station: Option<String>,
    pub is_anonymous: bool,
    pub sign_with_wallet: bool,
    // ── New geolocation fields from RobustGeolocation (all optional) ──────────
    pub city: Option<String>,              // e.g. "Nairobi"
    pub region: Option<String>,            // e.g. "Nairobi County"
    pub country: Option<String>,           // e.g. "Kenya"
    pub location_accuracy: Option<String>, // "high" | "medium" | "low" | "none"
    pub location_source: Option<String>,   // "gps" | "ip" | "browser-cached" | "failed"
    pub proxy_detected: Option<bool>,      // true if VPN/proxy was detected
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceUpdate {
    pub police_case_id: Option<String>,
    pub police_station: Option<String>,
    pub reported_to_police: Option<bool>,
    pub status: Option<EvidenceStatus>,
    pub needs_attention: Option<bool>,
    pub description: Option<String>,
    pub vehicle_details: Option<VehicleDetails>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EvidenceSearchFilters {
    pub query: Option<String>,
    pub county: Option<String>,
    pub incident_type: Option<String>,
    pub emergency_level: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub reported_to_police: Option<bool>,
    pub signed_only: Option<bool>,
    pub status: Option<String>,
    pub sort_by: Option<String>,
    pub page: u32,
    pub limit: u32,
    pub needs_attention: Option<bool>,
    pub uploader_id: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceSearchResponse {
    pub evidence: Vec<Evidence>,
    pub summaries: Vec<EvidenceSummary>,
    pub total: u64,
    pub page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_evidence: u64,
    pub urgent_count: u64,
    pub reported_count: u64,
    pub needs_attention_count: u64,
    pub today_count: u64,
    pub by_county: Vec<CountyStats>,
    pub by_type: Vec<IncidentTypeStats>,
}

#[derive(Debug, Serialize)]
pub struct CountyStats {
    pub county: String,
    pub count: i64,
    pub urgent: i64,
}

#[derive(Debug, Serialize)]
pub struct IncidentTypeStats {
    pub incident_type: IncidentType,
    pub count: i64,
}

// ==================== MEDIA MODELS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub location_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ContentType {
    VideoClip,
    Movie,
    Series,
    Documentary,
    ShortFilm,
    LiveStream,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::VideoClip => "VideoClip",
            ContentType::Movie => "Movie",
            ContentType::Series => "Series",
            ContentType::Documentary => "Documentary",
            ContentType::ShortFilm => "ShortFilm",
            ContentType::LiveStream => "LiveStream",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum PrivacyLevel {
    Public,
    Private,
    Unlisted,
    Restricted,
}

impl PrivacyLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrivacyLevel::Public => "Public",
            PrivacyLevel::Private => "Private",
            PrivacyLevel::Unlisted => "Unlisted",
            PrivacyLevel::Restricted => "Restricted",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaContent {
    pub id: String,
    pub title: String,
    pub description: String,
    pub synopsis: String,
    pub content_type: ContentType,
    pub category: String,
    pub tags: Vec<String>,
    pub uploader_id: String,
    pub uploader_email: String,
    pub geo_location: Option<GeoLocation>,
    pub privacy_level: PrivacyLevel,
    pub storj_url: String,
    pub storj_bucket: String,
    pub storj_key: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: Option<u32>,
    pub file_size_bytes: u64,
    pub mime_type: String,
    pub wallet_signature: Option<String>,
    pub wallet_address: Option<String>,
    pub views_count: u64,
    pub likes_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaUploadRequest {
    pub title: String,
    pub description: String,
    pub synopsis: String,
    pub content_type: String,
    pub category: String,
    pub tags: String,
    pub privacy_level: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaSearchFilters {
    pub query: Option<String>,
    pub category: Option<String>,
    pub content_type: Option<String>,
    pub uploader_id: Option<String>,
    pub privacy_level: Option<String>,
    pub min_latitude: Option<f64>,
    pub max_latitude: Option<f64>,
    pub min_longitude: Option<f64>,
    pub max_longitude: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub sort_by: Option<String>,
    pub signed_only: Option<bool>,
    pub page: u32,
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaSearchResponse {
    pub content: Vec<MediaContent>,
    pub total: u64,
    pub page: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserLibrary {
    pub user_id: String,
    pub watch_history: Vec<WatchHistory>,
    pub favorites: Vec<String>,
    pub playlists: Vec<Playlist>,
    pub uploaded_content: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchHistory {
    pub media_id: String,
    pub timestamp: DateTime<Utc>,
    pub progress_seconds: u32,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub media_ids: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformStats {
    pub total_content: u64,
    pub total_views: u64,
    pub total_likes: u64,
    pub total_users: u64,
    pub signed_content: u64,
    pub total_watch_time_seconds: u64,
    pub top_categories: Vec<CategoryStats>,
    pub recent_uploads: Vec<MediaContent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CategoryStats {
    pub category: String,
    pub count: u64,
    pub total_views: u64,
    pub total_likes: u64,
}

// ==================== PROFILE MODELS ====================

// In models.rs - Update ProfileCompletionForm
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProfileCompletionForm {
    pub account_type: String,
    pub business_name: Option<String>,
    pub geo_latitude: Option<f64>,
    pub geo_longitude: Option<f64>,
    // Kenya fields
    pub phone_number: Option<String>,
    pub county: Option<String>,
    // Optional email for wallet users
    pub email: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfileCompletionResponse {
    pub success: bool,
    pub message: String,
    pub redirect_to: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfileStatusResponse {
    pub is_profile_complete: bool,
    pub missing_fields: Vec<String>,
    pub current_account_type: Option<String>,
}

// ==================== API REQUEST/RESPONSE MODELS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            message: "Success".to_string(),
            data: Some(data),
        }
    }
    
    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BrowseQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub content_type: Option<String>,
    pub sort_by: Option<String>,
    pub signed_only: Option<bool>,
    pub page: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WatchHistoryRequest {
    pub media_id: String,
    pub progress_seconds: u32,
    pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlaylistRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchSignRequest {
    pub content_ids: Vec<String>,
    pub chain: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadMediaRequest {
    pub title: String,
    pub description: String,
    pub synopsis: String,
    pub content_type: String,
    pub category: String,
    pub tags: String,
    pub privacy_level: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub location_name: Option<String>,
    pub sign_with_wallet: Option<bool>,
}

// ==================== VERIFICATION MODELS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentVerificationResult {
    pub content_id: String,
    pub title: String,
    pub is_valid: bool,
    pub issues: Vec<String>,
    pub wallet_signed: bool,
    pub wallet_address: Option<String>,
    pub signature_valid: bool,
    pub hash_match: bool,
    pub timestamp: DateTime<Utc>,
    pub verification_hash: String,
}

// ==================== WALLET AUTH MODELS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletLoginChallengeRequest {
    pub wallet_address: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletChallengeResponse {
    pub challenge: String,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletLoginRequest {
    pub wallet_address: String,
    pub signature: String,
    pub wallet_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletLoginResponse {
    pub success: bool,
    pub message: String,
    pub requires_registration: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectWalletForm {
    pub wallet_address: String,
    pub chain: String,
    pub wallet_type: String,
    pub signature: String,
    pub public_key: Option<String>,
    pub hidden_email: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConnectWalletResponse {
    pub success: bool,
    pub message: String,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletLoginFormData {
    pub wallet_address: String,
    pub chain: String,
    pub message: String,
}

// ==================== AUTH FORM MODELS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoginForm {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisterForm {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VerifyEmailForm {
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SetupPasswordForm {
    pub email: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ForgotPasswordForm {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResetPasswordForm {
    pub token: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletChallengeRequest {
    pub email: String,
    pub wallet_address: String,
    pub chain: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletConnectRequest {
    pub wallet_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletVerifyRequest {
    pub wallet_address: String,
    pub signature: String,
    pub wallet_type: String,
}

// ==================== TARGET PHOTO MODELS ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetCategory {
    Person,
    Vehicle,
    Object,
    Location,
    Other,
}

impl TargetCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetCategory::Person => "person",
            TargetCategory::Vehicle => "vehicle",
            TargetCategory::Object => "object",
            TargetCategory::Location => "location",
            TargetCategory::Other => "other",
        }
    }
    
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "person" => Some(TargetCategory::Person),
            "vehicle" => Some(TargetCategory::Vehicle),
            "object" => Some(TargetCategory::Object),
            "location" => Some(TargetCategory::Location),
            "other" => Some(TargetCategory::Other),
            _ => None,
        }
    }
}

impl std::fmt::Display for TargetCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPhoto {
    pub id: String,
    pub evidence_id: String,
    pub target_number: i32,
    pub filename: String,
    pub mime_type: String,
    pub file_size: u64,
    pub description: Option<String>,
    pub category: TargetCategory,
    pub confidence_score: i32,
    pub storj_url: String,
    pub storj_key: String,
    pub hash: String,
    /// 16-char hex perceptual hash — None until face sidecar processes the image
    pub phash: Option<String>,
    /// true = auto-created from multi-face split, not manually selected by uploader
    pub auto_generated: bool,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetUploadRequest {
    pub evidence_id: String,
    pub photos: Vec<TargetPhotoData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetPhotoData {
    pub filename: String,
    pub mime_type: String,
    pub data_base64: String,
    pub description: Option<String>,
    pub category: String,
    pub confidence_score: i32,
}

// Add this to the end of your models.rs file, replacing the incorrect structs:

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceCompleteForm {
    pub evidence_id: String,
    pub complete_notes: Option<String>,
    pub is_complete: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceSignForm {
    pub evidence_id: String,
    pub wallet_address: String,
    pub chain: Option<String>,
    pub signature: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoliceReportForm {
    pub evidence_id: String,
    pub police_station: String,
    pub report_number: String,
    pub officer_name: Option<String>,
    pub contact_number: Option<String>,
    pub additional_notes: Option<String>,
}

// Add to src/models.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct EvidenceLocationData {
    pub id: String,
    pub evidence_number: String,
    pub title: String,
    pub emergency_level: EmergencyLevel,
    pub incident_type: IncidentType,
    pub county: String,
    pub latitude: f64,
    pub longitude: f64,
    pub incident_time: DateTime<Utc>,
    pub status: EvidenceStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatisticsResponse {
    pub total_targets: i64,
    pub targets_size_bytes: i64,
    pub total_evidence: i64,
    pub evidence_size_bytes: i64,
    pub total_audit: i64,
}


// Add to models.rs

#[derive(Debug, Serialize, Deserialize)]
pub struct EvidenceLocationFilters {
    pub county: Option<String>,
    pub emergency_level: Option<String>,
    pub incident_type: Option<String>,
    pub status: Option<String>,
    pub reported_to_police: Option<bool>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub search_query: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceMapStatistics {
    pub total_evidence: i64,
    pub urgent_count: i64,
    pub reported_count: i64,
    pub county_stats: HashMap<String, (i64, i64)>, // county -> (total, urgent)
    pub incident_stats: HashMap<String, i64>,
}


#[derive(Debug, Serialize)]
pub struct EvidenceLocationDataSup {
    pub id: String,
    pub evidence_number: String,
    pub title: String,
    pub emergency_level: EmergencyLevel,
    pub incident_type: IncidentType,
    pub county: String,
    pub latitude: f64,
    pub longitude: f64,
    pub incident_time: DateTime<Utc>,
    pub status: EvidenceStatus,
    // Add these missing fields:
    pub reported_to_police: bool,
    pub police_case_id: Option<String>,
    pub uploader_email: String,
    pub created_at: DateTime<Utc>,
    pub media_count: i32,
    pub needs_attention: bool,
}

// Add these structs to your models.rs

#[derive(Debug, Deserialize)]
pub struct ProfileUpdateForm {
    pub phone_number: Option<String>,
    pub county: Option<String>,
    pub business_name: Option<String>,
    pub account_type: Option<String>,
    pub id_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PasswordChangeForm {
    pub current_password: String,
    pub new_password: String,
    pub confirm_password: String,
}

#[derive(Debug, Deserialize)]
pub struct WalletDisconnectForm {
    pub wallet_address: String,
}


// Add to src/models.rs

#[derive(Debug, Serialize)]
pub struct ChartData {
    pub collated: i64,
    pub reported: i64,
    pub submitted: i64,
    pub draft: i64,
    pub urgent: i64,
    pub signed: i64,
    pub others: i64,
}

#[derive(Debug, Serialize)]
pub struct MediaStorageStats {
    pub media: i64,
    pub scenes: i64,
    pub profiles: i64,
    pub evidence: i64,
    pub target: i64,
}

#[derive(Debug, Serialize)]
pub struct StorageSizeData {
    pub media: i64,
    pub evidence: i64,
    pub target: i64,
}

// ==================== LINKED EVIDENCE & NOTIFICATIONS ====================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkedEvidenceRecord {
    pub link_id: String,
    pub evidence_id_1: String,
    pub evidence_id_2: String,
    pub link_type: String,
    pub link_reason: String,
    pub matched_target_hash: String,
    pub confidence_score: i32,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<String>,
    // Denormalized fields for convenience
    pub other_evidence_id: String,
    pub other_evidence_number: String,
    pub other_title: String,
    pub other_emergency_level: String,
    pub other_uploader_email: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotificationRecord {
    pub id: String,
    pub user_id: String,
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub evidence_id: Option<String>,
    pub linked_evidence_id: Option<String>,
    pub target_hash: Option<String>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}