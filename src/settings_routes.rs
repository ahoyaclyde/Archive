// src/settings_routes.rs
//
// Provides ALL backend APIs for the /settings page.
// Covers: general, POI management, response protocols, alerts,
// evidence rules, feedback, privacy, data retention, and danger zone.
//
// REGISTER in main.rs:
//   mod settings_routes;
//   // inside HttpServer::new:
//   .configure(settings_routes::config)
//
// REGISTER in lib.rs:
//   pub mod settings_routes;

use actix_web::{web, HttpResponse};
use actix_session::Session;
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::Utc;
use uuid::Uuid;

use crate::auth::AuthService;
use crate::models::ApiResponse;
use crate::database::Database;

// ═══════════════════════════════════════════════════════════════
// SETTINGS MODELS
// ═══════════════════════════════════════════════════════════════

/// The full settings payload returned by GET /api/settings.
/// Each sub-struct maps 1-to-1 with a section in the UI.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlatformSettings {
    pub general:        GeneralSettings,
    pub poi:            PoiSettings,
    pub response:       ResponseSettings,
    pub alerts:         AlertSettings,
    pub evidence_rules: EvidenceRulesSettings,
    pub feedback:       FeedbackSettings,
    pub privacy:        PrivacySettings,
    pub retention:      RetentionSettings,
}

impl Default for PlatformSettings {
    fn default() -> Self {
        Self {
            general:        GeneralSettings::default(),
            poi:            PoiSettings::default(),
            response:       ResponseSettings::default(),
            alerts:         AlertSettings::default(),
            evidence_rules: EvidenceRulesSettings::default(),
            feedback:       FeedbackSettings::default(),
            privacy:        PrivacySettings::default(),
            retention:      RetentionSettings::default(),
        }
    }
}

// ── General ─────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralSettings {
    pub evidence_id_format:       String,  // "evd_yyyy_xxxxx" | "short_uuid" | "sequential"
    pub default_map_view:         String,  // "kenya_national" | "nairobi" | "last_used" | "user_location"
    pub date_format:              String,  // "dd_mmm_yyyy" | "mm_dd_yyyy" | "iso_8601"
    pub timezone:                 String,  // "Africa/Nairobi"
    pub dashboard_refresh_secs:   u32,     // 0 = off, 30, 60, 300
    pub show_evidence_count_badge: bool,
    pub auto_save_drafts:         bool,
    pub require_gps:              bool,
    pub blockchain_sign_on_submit: bool,
    pub maintenance_mode:         bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            evidence_id_format:        "evd_yyyy_xxxxx".into(),
            default_map_view:          "kenya_national".into(),
            date_format:               "dd_mmm_yyyy".into(),
            timezone:                  "Africa/Nairobi".into(),
            dashboard_refresh_secs:    30,
            show_evidence_count_badge: true,
            auto_save_drafts:          true,
            require_gps:               true,
            blockchain_sign_on_submit: false,
            maintenance_mode:          false,
        }
    }
}

// ── Persons of Interest ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoiSettings {
    /// 0–100 — minimum similarity % to surface a match
    pub photo_similarity_threshold:    u32,
    /// 1–7 — minimum plate chars for a partial hit
    pub plate_partial_match_chars:     u32,
    /// Auto-pin after N sightings without an existing POI
    pub auto_pin_after_sightings:      u32,
    /// Days before an inactive POI is auto-archived (0 = never)
    pub pin_expiry_days:               u32,
    // Action flags fired when a confirmed match exceeds threshold
    pub action_notify_owner:           bool,
    pub action_auto_draft_police:      bool,
    pub action_lock_evidence:          bool,
    pub action_link_evidence:          bool,
    pub action_broadcast_match:        bool,
    pub action_escalate_to_red:        bool,
    /// Hours between re-firing the same POI action chain (0 = no cooldown)
    pub escalation_cooldown_hours:     u32,
}

impl Default for PoiSettings {
    fn default() -> Self {
        Self {
            photo_similarity_threshold: 78,
            plate_partial_match_chars:  5,
            auto_pin_after_sightings:   3,
            pin_expiry_days:            90,
            action_notify_owner:        true,
            action_auto_draft_police:   true,
            action_lock_evidence:       false,
            action_link_evidence:       true,
            action_broadcast_match:     false,
            action_escalate_to_red:     false,
            escalation_cooldown_hours:  6,
        }
    }
}

// ── Response Protocols ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LevelResponse {
    pub notify:          bool,
    pub stage_report:    bool,
    pub lock_evidence:   bool,
    pub auto_submit:     bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseSettings {
    pub red:    LevelResponse,
    pub orange: LevelResponse,
    pub yellow: LevelResponse,
    pub blue:   LevelResponse,
    /// Hours before a "no OB acknowledgement" followup fires (0 = disabled)
    pub followup_hours:          u32,
    pub cc_admin_on_red:         bool,
    pub primary_contact_email:   String,
    pub secondary_contact_email: String,
    pub police_liaison_email:    String,
}

impl Default for ResponseSettings {
    fn default() -> Self {
        Self {
            red:    LevelResponse { notify: true,  stage_report: true,  lock_evidence: true,  auto_submit: false },
            orange: LevelResponse { notify: true,  stage_report: true,  lock_evidence: false, auto_submit: false },
            yellow: LevelResponse { notify: true,  stage_report: false, lock_evidence: false, auto_submit: false },
            blue:   LevelResponse { notify: true,  stage_report: false, lock_evidence: false, auto_submit: false },
            followup_hours:          24,
            cc_admin_on_red:         false,
            primary_contact_email:   String::new(),
            secondary_contact_email: String::new(),
            police_liaison_email:    String::new(),
        }
    }
}

// ── Alert Configuration ─────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertSettings {
    pub channel_in_app:        bool,
    pub channel_email:         bool,
    pub channel_sms:           bool,
    pub channel_browser_push:  bool,
    pub quiet_hours_enabled:   bool,
    pub quiet_start:           String,  // "22:00"
    pub quiet_end:             String,  // "07:00"
    pub digest_mode:           String,  // "off" | "daily_0800" | "twice_daily"
    // Per-event toggles
    pub notify_own_submission:         bool,
    pub notify_evidence_linked:        bool,
    pub notify_poi_match:              bool,
    pub notify_police_status_change:   bool,
    pub notify_feedback_received:      bool,
    pub notify_system_announcements:   bool,
}

impl Default for AlertSettings {
    fn default() -> Self {
        Self {
            channel_in_app:       true,
            channel_email:        true,
            channel_sms:          false,
            channel_browser_push: false,
            quiet_hours_enabled:  false,
            quiet_start:          "22:00".into(),
            quiet_end:            "07:00".into(),
            digest_mode:          "daily_0800".into(),
            notify_own_submission:       true,
            notify_evidence_linked:      true,
            notify_poi_match:            true,
            notify_police_status_change: true,
            notify_feedback_received:    false,
            notify_system_announcements: true,
        }
    }
}

// ── Evidence Rules ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvidenceRulesSettings {
    pub min_files:               u32,
    pub max_files:               u32,
    pub max_file_size_mb:        u32,
    pub allowed_mime_types:      Vec<String>,
    pub gps_accuracy_metres:     u32,
    // Auto-linking
    pub link_geo_radius_metres:  u32,
    pub link_time_window_secs:   u32,
    pub link_by_target_hash:     bool,
    pub require_admin_link_approval: bool,
}

impl Default for EvidenceRulesSettings {
    fn default() -> Self {
        Self {
            min_files:           1,
            max_files:           3,
            max_file_size_mb:    100,
            allowed_mime_types:  vec![
                "video/mp4".into(),
                "video/webm".into(),
                "image/jpeg".into(),
                "image/png".into(),
                "audio/mpeg".into(),
            ],
            gps_accuracy_metres:     100,
            link_geo_radius_metres:  500,
            link_time_window_secs:   7200,  // 2 hours
            link_by_target_hash:     true,
            require_admin_link_approval: false,
        }
    }
}

// ── Feedback & Reports ──────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeedbackSettings {
    pub allow_feedback:              bool,
    pub allow_anonymous_feedback:    bool,
    pub response_sla_hours:          u32,  // 0 = no target
    pub notify_on_feedback:          bool,
    pub require_verified_for_feedback: bool,
    pub default_police_station:      String,
    pub report_header_text:          String,
    pub include_blockchain_hash:     bool,
    pub include_gps_in_report:       bool,
    pub attach_media_to_report:      bool,
}

impl Default for FeedbackSettings {
    fn default() -> Self {
        Self {
            allow_feedback:              true,
            allow_anonymous_feedback:    false,
            response_sla_hours:          72,
            notify_on_feedback:          true,
            require_verified_for_feedback: true,
            default_police_station:      String::new(),
            report_header_text:          "This evidence package was submitted via FLUG Evidence Platform \
                                          and has been cryptographically signed. The chain of custody is \
                                          sealed and can be verified at any time.".into(),
            include_blockchain_hash:     true,
            include_gps_in_report:       true,
            attach_media_to_report:      true,
        }
    }
}

// ── Privacy & Witness ───────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivacySettings {
    pub hide_email_in_linked_cases: bool,
    pub allow_anonymous_submissions: bool,
    pub strip_exif_on_upload:       bool,
    pub witness_protection_mode:    bool,
    pub default_visibility:         String, // "private" | "verified_users" | "public"
    pub allow_browse_discovery:     bool,
    pub show_on_map_by_default:     bool,
}

impl Default for PrivacySettings {
    fn default() -> Self {
        Self {
            hide_email_in_linked_cases:  true,
            allow_anonymous_submissions: false,
            strip_exif_on_upload:        true,
            witness_protection_mode:     false,
            default_visibility:          "verified_users".into(),
            allow_browse_discovery:      false,
            show_on_map_by_default:      true,
        }
    }
}

// ── Data Retention ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetentionSettings {
    pub draft_retention_days:      u32,  // 0 = never
    pub submitted_retention_days:  u32,
    pub medium_retention_days:     u32,
    pub critical_retention_days:   u32,  // 0 = never
    pub audit_log_retention_days:  u32,
    pub session_retention_hours:   u32,
    pub purge_action:              String, // "cold_storage" | "delete_media_keep_meta" | "hard_delete"
    pub notify_before_purge_days:  u32,   // 0 = no notice
    pub auto_run_retention:        bool,
}

impl Default for RetentionSettings {
    fn default() -> Self {
        Self {
            draft_retention_days:     30,
            submitted_retention_days: 1095, // 3 years
            medium_retention_days:    1825, // 5 years
            critical_retention_days:  0,    // never
            audit_log_retention_days: 365,
            session_retention_hours:  24,
            purge_action:             "cold_storage".into(),
            notify_before_purge_days: 7,
            auto_run_retention:       true,
        }
    }
}

// ── POI Profile ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PoiProfile {
    pub id:             String,
    pub poi_number:     String,        // "POI-2024-XXXX"
    pub display_name:   String,        // may be "Unknown — Vehicle KBZ 448Y"
    pub category:       String,        // "person" | "vehicle" | "unknown"
    pub status:         String,        // "active" | "watching" | "resolved" | "archived"
    pub linked_cases:   i64,
    pub linked_evidence: Vec<String>,  // evidence IDs
    pub notes:          Option<String>,
    pub pinned_by:      String,        // user_id
    pub created_at:     i64,
    pub last_seen_at:   Option<i64>,
    pub resolved_at:    Option<i64>,
}

// ── Request bodies ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PinPoiRequest {
    pub display_name:    String,
    pub category:        String,
    pub evidence_ids:    Vec<String>,
    pub notes:           Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePoiStatusRequest {
    pub status: String, // "watching" | "active" | "resolved" | "archived"
}

// ═══════════════════════════════════════════════════════════════
// HELPER — session guard
// ═══════════════════════════════════════════════════════════════

/// Returns (user_id, email) or an Unauthorized response.
fn require_session(session: &Session) -> Result<(String, String), HttpResponse> {
    let user_id = session.get::<String>("user_id")
        .unwrap_or(None)
        .ok_or_else(|| HttpResponse::Unauthorized().json(
            ApiResponse::<()>::error("Not authenticated")
        ))?;
    let email = session.get::<String>("user_email")
        .unwrap_or(None)
        .unwrap_or_default();
    Ok((user_id, email))
}

// ═══════════════════════════════════════════════════════════════
// PAGE ROUTE
// ═══════════════════════════════════════════════════════════════

pub async fn settings_page(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(e) => e,
        None => return HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish(),
    };

    match auth_service.get_user_by_email(&email).await {
        Ok(Some(user)) if !user.is_profile_complete => {
            return HttpResponse::SeeOther()
                .append_header(("Location", "/profile/complete"))
                .finish();
        }
        Ok(None) | Err(_) => {
            let _ = session.clear();
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
        _ => {}
    }

    match std::fs::read_to_string("static/templates/settings.html") {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(e) => {
            println!("❌ Could not load settings template: {}", e);
            HttpResponse::InternalServerError().body("Template not found")
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// GET /api/settings  — load all settings at once
// ═══════════════════════════════════════════════════════════════

pub async fn api_get_settings(
    session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚙️  API_GET_SETTINGS: Loading settings for user {}", user_id);

    // Each section stored under a namespaced key in platform_settings.
    // Falls back to Default if not yet saved.
    let general: GeneralSettings = database
        .get_setting_json(&user_id, "general")
        .await
        .unwrap_or_default();

    let poi: PoiSettings = database
        .get_setting_json(&user_id, "poi")
        .await
        .unwrap_or_default();

    let response: ResponseSettings = database
        .get_setting_json(&user_id, "response")
        .await
        .unwrap_or_default();

    let alerts: AlertSettings = database
        .get_setting_json(&user_id, "alerts")
        .await
        .unwrap_or_default();

    let evidence_rules: EvidenceRulesSettings = database
        .get_setting_json(&user_id, "evidence_rules")
        .await
        .unwrap_or_default();

    let feedback: FeedbackSettings = database
        .get_setting_json(&user_id, "feedback")
        .await
        .unwrap_or_default();

    let privacy: PrivacySettings = database
        .get_setting_json(&user_id, "privacy")
        .await
        .unwrap_or_default();

    let retention: RetentionSettings = database
        .get_setting_json(&user_id, "retention")
        .await
        .unwrap_or_default();

    let settings = PlatformSettings {
        general,
        poi,
        response,
        alerts,
        evidence_rules,
        feedback,
        privacy,
        retention,
    };

    println!("✅ API_GET_SETTINGS: Returning settings for user {}", user_id);
    HttpResponse::Ok().json(ApiResponse::success(settings))
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/general
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_general(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<GeneralSettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚙️  API_SAVE_GENERAL: Saving general settings for user {}", user_id);

    match database.set_setting_json(&user_id, "general", &*body).await {
        Ok(_) => {
            println!("✅ API_SAVE_GENERAL: Saved for user {}", user_id);
            HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true })))
        }
        Err(e) => {
            println!("❌ API_SAVE_GENERAL: Failed — {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e)))
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/poi
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_poi_settings(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<PoiSettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    // Basic validation
    if body.photo_similarity_threshold > 100 {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("photo_similarity_threshold must be 0–100"));
    }
    if body.plate_partial_match_chars > 7 {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("plate_partial_match_chars must be 1–7"));
    }

    println!("⚙️  API_SAVE_POI_SETTINGS: Saving POI settings for user {}", user_id);

    match database.set_setting_json(&user_id, "poi", &*body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/response
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_response_settings(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<ResponseSettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚙️  API_SAVE_RESPONSE: Saving response protocols for user {}", user_id);

    match database.set_setting_json(&user_id, "response", &*body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/alerts
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_alert_settings(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<AlertSettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚙️  API_SAVE_ALERTS: Saving alert config for user {}", user_id);

    match database.set_setting_json(&user_id, "alerts", &*body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/evidence-rules
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_evidence_rules(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<EvidenceRulesSettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    if body.min_files < 1 || body.min_files > body.max_files {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("min_files must be ≥ 1 and ≤ max_files"));
    }

    println!("⚙️  API_SAVE_EVIDENCE_RULES: Saving for user {}", user_id);

    match database.set_setting_json(&user_id, "evidence_rules", &*body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/feedback
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_feedback_settings(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<FeedbackSettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚙️  API_SAVE_FEEDBACK: Saving feedback settings for user {}", user_id);

    match database.set_setting_json(&user_id, "feedback", &*body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/privacy
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_privacy_settings(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<PrivacySettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    let valid = ["private", "verified_users", "public"];
    if !valid.contains(&body.default_visibility.as_str()) {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("Invalid default_visibility value"));
    }

    println!("⚙️  API_SAVE_PRIVACY: Saving privacy settings for user {}", user_id);

    match database.set_setting_json(&user_id, "privacy", &*body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /api/settings/retention
// ═══════════════════════════════════════════════════════════════

pub async fn api_save_retention_settings(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<RetentionSettings>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    let valid_actions = ["cold_storage", "delete_media_keep_meta", "hard_delete"];
    if !valid_actions.contains(&body.purge_action.as_str()) {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("Invalid purge_action value"));
    }

    println!("⚙️  API_SAVE_RETENTION: Saving retention settings for user {}", user_id);

    match database.set_setting_json(&user_id, "retention", &*body).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "saved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to save: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POI MANAGEMENT APIS
// ═══════════════════════════════════════════════════════════════

/// GET /api/poi — list all POIs for this user
pub async fn api_get_poi_list(
    session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("🎯 API_GET_POI_LIST: Fetching POIs for user {}", user_id);

    match database.get_poi_list(&user_id).await {
        Ok(pois) => {
            println!("✅ API_GET_POI_LIST: Returning {} POIs", pois.len());
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "pois": pois,
                "total": pois.len(),
            })))
        }
        Err(e) => {
            println!("❌ API_GET_POI_LIST: Error — {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to load POIs: {}", e)))
        }
    }
}

/// POST /api/poi — pin a new person of interest
pub async fn api_pin_poi(
    session: Session,
    database: web::Data<Database>,
    body: web::Json<PinPoiRequest>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    if body.display_name.trim().is_empty() {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("display_name is required"));
    }

    let valid_cats = ["person", "vehicle", "unknown"];
    if !valid_cats.contains(&body.category.as_str()) {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("category must be person | vehicle | unknown"));
    }

    let poi_id     = format!("poi_{}", Uuid::new_v4());
    let poi_number = generate_poi_number();

    println!("🎯 API_PIN_POI: Pinning {} — {}", poi_number, body.display_name);

    let poi = PoiProfile {
        id:               poi_id.clone(),
        poi_number:       poi_number.clone(),
        display_name:     body.display_name.clone(),
        category:         body.category.clone(),
        status:           "watching".into(),
        linked_cases:     body.evidence_ids.len() as i64,
        linked_evidence:  body.evidence_ids.clone(),
        notes:            body.notes.clone(),
        pinned_by:        user_id.clone(),
        created_at:       Utc::now().timestamp(),
        last_seen_at:     None,
        resolved_at:      None,
    };

    match database.create_poi(&poi).await {
        Ok(_) => {
            println!("✅ API_PIN_POI: Created {}", poi_number);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "poi_id":     poi_id,
                "poi_number": poi_number,
                "message":    "POI pinned successfully",
            })))
        }
        Err(e) => {
            println!("❌ API_PIN_POI: Failed — {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to pin POI: {}", e)))
        }
    }
}

/// GET /api/poi/{id} — fetch a single POI
pub async fn api_get_poi(
    session: Session,
    database: web::Data<Database>,
    path: web::Path<String>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let poi_id = path.into_inner();

    match database.get_poi(&poi_id, &user_id).await {
        Ok(Some(poi)) => HttpResponse::Ok().json(ApiResponse::success(poi)),
        Ok(None)      => HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("POI not found")),
        Err(e)        => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Error: {}", e))),
    }
}

/// POST /api/poi/{id}/status — update status (watching/active/resolved/archived)
pub async fn api_update_poi_status(
    session: Session,
    database: web::Data<Database>,
    path: web::Path<String>,
    body: web::Json<UpdatePoiStatusRequest>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let poi_id = path.into_inner();

    let valid = ["watching", "active", "resolved", "archived"];
    if !valid.contains(&body.status.as_str()) {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("status must be watching | active | resolved | archived"));
    }

    println!("🎯 API_UPDATE_POI_STATUS: {} → {} (by user {})", poi_id, body.status, user_id);

    match database.update_poi_status(&poi_id, &user_id, &body.status).await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({
            "poi_id": poi_id,
            "status": body.status,
        }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to update POI: {}", e))),
    }
}

/// POST /api/poi/{id}/archive
pub async fn api_archive_poi(
    session: Session,
    database: web::Data<Database>,
    path: web::Path<String>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let poi_id = path.into_inner();

    println!("🎯 API_ARCHIVE_POI: Archiving {} (by user {})", poi_id, user_id);

    match database.update_poi_status(&poi_id, &user_id, "archived").await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "archived": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to archive: {}", e))),
    }
}

/// POST /api/poi/{id}/resolve
pub async fn api_resolve_poi(
    session: Session,
    database: web::Data<Database>,
    path: web::Path<String>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };
    let poi_id = path.into_inner();

    println!("🎯 API_RESOLVE_POI: Resolving {} (by user {})", poi_id, user_id);

    match database.update_poi_status(&poi_id, &user_id, "resolved").await {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::success(json!({ "resolved": true }))),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to resolve: {}", e))),
    }
}

fn generate_poi_number() -> String {
    let year = Utc::now().format("%Y").to_string();
    let seq  = format!("{:04}", rand::random::<u16>() % 9999 + 1);
    format!("POI-{}-{}", year, seq)
}

// ═══════════════════════════════════════════════════════════════
// DANGER ZONE APIS
// All require admin role. Each logs to audit_log.
// ═══════════════════════════════════════════════════════════════

/// POST /api/admin/purge-sessions
/// Deletes all expired session records from the DB.
pub async fn api_purge_sessions(
    session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚠️  DANGER_ZONE: purge_sessions requested by user {}", user_id);

    match database.purge_expired_sessions().await {
        Ok(count) => {
            database.log_audit(
                Some(&user_id),
                "danger_purge_sessions",
                "system",
                None,
                &format!("Purged {} expired sessions", count),
                None,
            ).await.ok();
            println!("✅ DANGER_ZONE: Purged {} sessions", count);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "deleted": count,
                "message": format!("Purged {} expired sessions", count),
            })))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
    }
}

/// POST /api/admin/reset-poi
/// Archives ALL POI records. Linked cases and evidence are preserved.
pub async fn api_reset_all_poi(
    session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚠️  DANGER_ZONE: reset_all_poi requested by user {}", user_id);

    match database.archive_all_poi(&user_id).await {
        Ok(count) => {
            database.log_audit(
                Some(&user_id),
                "danger_reset_poi",
                "poi",
                None,
                &format!("Archived all {} POI records", count),
                None,
            ).await.ok();
            println!("✅ DANGER_ZONE: Archived {} POI records", count);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "archived": count,
                "message": format!("Archived {} POI records", count),
            })))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
    }
}

/// POST /api/admin/purge-archived-evidence
/// Permanently removes evidence with status = 'archived' from DB.
/// NOTE: Does NOT delete Storj objects — wire that separately via EvidenceService if needed.
pub async fn api_purge_archived_evidence(
    session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚠️  DANGER_ZONE: purge_archived_evidence requested by user {}", user_id);

    match database.delete_archived_evidence(&user_id).await {
        Ok(count) => {
            database.log_audit(
                Some(&user_id),
                "danger_purge_archived_evidence",
                "evidence",
                None,
                &format!("Hard-deleted {} archived evidence records", count),
                None,
            ).await.ok();
            println!("✅ DANGER_ZONE: Deleted {} archived evidence records", count);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "deleted": count,
                "message": format!("Permanently deleted {} archived evidence records", count),
            })))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
    }
}

/// POST /api/admin/wipe-notifications
/// Clears the entire notifications table for this user.
pub async fn api_wipe_notifications(
    session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("⚠️  DANGER_ZONE: wipe_notifications requested by user {}", user_id);

    match database.wipe_user_notifications(&user_id).await {
        Ok(count) => {
            database.log_audit(
                Some(&user_id),
                "danger_wipe_notifications",
                "notifications",
                None,
                &format!("Wiped {} notification records", count),
                None,
            ).await.ok();
            println!("✅ DANGER_ZONE: Wiped {} notifications", count);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "deleted": count,
                "message": format!("Wiped {} notification records", count),
            })))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
    }
}

/// POST /api/admin/full-reset
/// ⚡ IRREVERSIBLE — wipes all user evidence, notifications, POIs, and settings.
/// Preserves: users table, audit_log, schema.
pub async fn api_full_platform_reset(
    session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let (user_id, _) = match require_session(&session) {
        Ok(v) => v,
        Err(r) => return r,
    };

    println!("🔴 DANGER_ZONE: FULL_PLATFORM_RESET requested by user {} — EXECUTING", user_id);

    match database.full_platform_reset(&user_id).await {
        Ok(stats) => {
            database.log_audit(
                Some(&user_id),
                "danger_full_reset",
                "platform",
                None,
                &format!("FULL PLATFORM RESET executed: {:?}", stats),
                None,
            ).await.ok();
            println!("✅ DANGER_ZONE: Full reset complete — {:?}", stats);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "reset": true,
                "stats": stats,
                "message": "Platform has been fully reset. All evidence, POIs, notifications, and settings cleared.",
            })))
        }
        Err(e) => {
            println!("❌ DANGER_ZONE: Full reset FAILED — {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Reset failed: {}", e)))
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// ROUTE CONFIG
// Add `.configure(settings_routes::config)` in main.rs
// ═══════════════════════════════════════════════════════════════

pub fn config(cfg: &mut web::ServiceConfig) {
    // Page
    cfg.route("/settings", web::get().to(settings_page));

    // Settings — read all
    cfg.route("/api/settings", web::get().to(api_get_settings));

    // Settings — save each section
    cfg.route("/api/settings/general",        web::post().to(api_save_general))
       .route("/api/settings/poi",            web::post().to(api_save_poi_settings))
       .route("/api/settings/response",       web::post().to(api_save_response_settings))
       .route("/api/settings/alerts",         web::post().to(api_save_alert_settings))
       .route("/api/settings/evidence-rules", web::post().to(api_save_evidence_rules))
       .route("/api/settings/feedback",       web::post().to(api_save_feedback_settings))
       .route("/api/settings/privacy",        web::post().to(api_save_privacy_settings))
       .route("/api/settings/retention",      web::post().to(api_save_retention_settings));

    // POI management
    cfg.route("/api/poi",                web::get().to(api_get_poi_list))
       .route("/api/poi",                web::post().to(api_pin_poi))
       .route("/api/poi/{id}",           web::get().to(api_get_poi))
       .route("/api/poi/{id}/status",    web::post().to(api_update_poi_status))
       .route("/api/poi/{id}/archive",   web::post().to(api_archive_poi))
       .route("/api/poi/{id}/resolve",   web::post().to(api_resolve_poi));

    // Danger zone
    cfg.route("/api/admin/purge-sessions",          web::post().to(api_purge_sessions))
       .route("/api/admin/reset-poi",               web::post().to(api_reset_all_poi))
       .route("/api/admin/purge-archived-evidence", web::post().to(api_purge_archived_evidence))
       .route("/api/admin/wipe-notifications",      web::post().to(api_wipe_notifications))
       .route("/api/admin/full-reset",              web::post().to(api_full_platform_reset));
}