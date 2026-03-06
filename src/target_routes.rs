// src/target_routes.rs
//
// Provides ALL backend APIs for target flag actions in the Target Profile modal.
// Each action persists to `target_flags` table, fires a notification, and writes
// an audit log entry.
//
// REGISTER in main.rs:
//   mod target_routes;
//   // inside HttpServer::new:
//   .configure(target_routes::config)
//
// REGISTER in lib.rs:
//   pub mod target_routes;

use actix_web::{web, HttpResponse};
use actix_session::Session;
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::database::Database;
use crate::models::ApiResponse;

// ═══════════════════════════════════════════════════════════════
// MODELS
// ═══════════════════════════════════════════════════════════════

/// Full persisted flag state returned by GET /targets/{id}/state
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetFlagState {
    pub target_id:   String,
    pub pinned:      bool,
    pub poi:         bool,
    pub watchlist:   bool,
    pub flagged:     bool,
    pub takedown:    bool,
    pub notes:       String,
    pub linked_cases: Vec<String>,
}

impl Default for TargetFlagState {
    fn default() -> Self {
        Self {
            target_id:    String::new(),
            pinned:       false,
            poi:          false,
            watchlist:    false,
            flagged:      false,
            takedown:     false,
            notes:        String::new(),
            linked_cases: Vec::new(),
        }
    }
}

/// Request body for toggle actions (pin / poi / watchlist / flag)
#[derive(Debug, Deserialize)]
pub struct ToggleRequest {
    pub target_id: Option<String>,
    pub active:    Option<bool>,
}

/// Request body for notes save
#[derive(Debug, Deserialize)]
pub struct NotesRequest {
    pub target_id: Option<String>,
    pub notes:     String,
}

/// Request body for link-case
#[derive(Debug, Deserialize)]
pub struct LinkCaseRequest {
    pub target_id: Option<String>,
    pub case_ref:  String,
}

/// Request body for takedown (body is optional — we use path param)
#[derive(Debug, Deserialize)]
pub struct TakedownRequest {
    pub target_id: Option<String>,
}

// ═══════════════════════════════════════════════════════════════
// TABLE BOOTSTRAP
// Called once from config() so the table always exists.
// ═══════════════════════════════════════════════════════════════

fn ensure_target_flags_table(db: &Database) {
    if let Ok(conn) = db.pool.get() {
        let _ = conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS target_flags (
                id               TEXT PRIMARY KEY,
                target_id        TEXT NOT NULL,
                user_id          TEXT NOT NULL,
                is_pinned        INTEGER NOT NULL DEFAULT 0,
                is_poi           INTEGER NOT NULL DEFAULT 0,
                is_watchlist     INTEGER NOT NULL DEFAULT 0,
                is_flagged       INTEGER NOT NULL DEFAULT 0,
                is_takedown      INTEGER NOT NULL DEFAULT 0,
                notes            TEXT NOT NULL DEFAULT '',
                linked_case_refs TEXT NOT NULL DEFAULT '[]',
                updated_at       INTEGER NOT NULL DEFAULT 0,
                UNIQUE(target_id, user_id)
            );
            CREATE INDEX IF NOT EXISTS idx_target_flags_target ON target_flags(target_id);
            CREATE INDEX IF NOT EXISTS idx_target_flags_user   ON target_flags(user_id);
        "#);
    }
}

// ═══════════════════════════════════════════════════════════════
// INTERNAL HELPERS
// ═══════════════════════════════════════════════════════════════

/// Pull session user_id or return 401
fn require_user_id(session: &Session) -> Result<String, HttpResponse> {
    match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => Ok(id),
        None => Err(HttpResponse::Unauthorized()
            .json(ApiResponse::<()>::error("Not authenticated"))),
    }
}

/// Fetch or create the flag row for (target_id, user_id).
/// Returns (state, evidence_id, evidence_number, category, description)
fn load_target_state(
    conn: &rusqlite::Connection,
    target_id: &str,
    user_id: &str,
) -> anyhow::Result<(TargetFlagState, String, String, String, String)> {
    // Upsert the flags row so it always exists
    let flag_id = format!("tf_{}_{}", target_id, user_id);
    conn.execute(
        r#"
        INSERT OR IGNORE INTO target_flags
            (id, target_id, user_id, updated_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![flag_id, target_id, user_id, Utc::now().timestamp()],
    )?;

    // Read flags
    let (is_pinned, is_poi, is_watchlist, is_flagged, is_takedown, notes, linked_json) =
        conn.query_row(
            r#"
            SELECT is_pinned, is_poi, is_watchlist, is_flagged, is_takedown,
                   notes, linked_case_refs
            FROM target_flags
            WHERE target_id = ?1 AND user_id = ?2
            "#,
            params![target_id, user_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)? != 0,
                    row.get::<_, i64>(1)? != 0,
                    row.get::<_, i64>(2)? != 0,
                    row.get::<_, i64>(3)? != 0,
                    row.get::<_, i64>(4)? != 0,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )?;

    let linked_cases: Vec<String> =
        serde_json::from_str(&linked_json).unwrap_or_default();

    // Fetch basic target metadata for notification enrichment
    let (evidence_id, evidence_number, category, description) = conn
        .query_row(
            r#"
            SELECT t.evidence_id,
                   COALESCE(e.evidence_number, 'Unknown'),
                   COALESCE(t.category, 'other'),
                   COALESCE(t.description, '')
            FROM targets t
            LEFT JOIN evidence e ON e.id = t.evidence_id
            WHERE t.id = ?1
            "#,
            params![target_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .unwrap_or_else(|_| {
            (String::new(), String::new(), "other".into(), String::new())
        });

    Ok((
        TargetFlagState {
            target_id: target_id.to_string(),
            pinned: is_pinned,
            poi: is_poi,
            watchlist: is_watchlist,
            flagged: is_flagged,
            takedown: is_takedown,
            notes,
            linked_cases,
        },
        evidence_id,
        evidence_number,
        category,
        description,
    ))
}

/// Update a single boolean flag column
fn update_flag_col(
    conn: &rusqlite::Connection,
    target_id: &str,
    user_id: &str,
    column: &str,   // must be a whitelisted literal — never user input
    value: bool,
) -> anyhow::Result<()> {
    // Column is always a hard-coded string from our handlers — safe to interpolate
    let sql = format!(
        r#"
        UPDATE target_flags
        SET {} = ?1, updated_at = ?2
        WHERE target_id = ?3 AND user_id = ?4
        "#,
        column
    );
    conn.execute(&sql, params![value as i64, Utc::now().timestamp(), target_id, user_id])?;
    Ok(())
}

/// Write a notification for a target action
async fn notify_target_action(
    db: &Database,
    user_id: &str,
    notif_type: &str,
    title: &str,
    message: &str,
    evidence_id: &str,
    target_hash: Option<&str>,
) {
    let eid = if evidence_id.is_empty() { None } else { Some(evidence_id) };
    let _ = db.create_notification(
        user_id,
        notif_type,
        title,
        message,
        eid,
        None,
        target_hash,
    ).await;
}

/// Write an audit log entry for a target action
async fn audit_target_action(
    db: &Database,
    user_id: &str,
    action_type: &str,
    target_id: &str,
    details: &str,
) {
    let _ = db.log_audit(
        Some(user_id),
        action_type,
        "target",
        Some(target_id),
        details,
        None,
    ).await;
}

// ═══════════════════════════════════════════════════════════════
// GET /targets/{id}/state
// Returns the full persisted flag state so the modal can initialise correctly.
// ═══════════════════════════════════════════════════════════════

pub async fn api_get_target_state(
    path:     web::Path<String>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    match load_target_state(&conn, &target_id, &user_id) {
        Ok((state, _, _, _, _)) => {
            HttpResponse::Ok().json(ApiResponse::success(json!({ "state": state })))
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed to load state: {}", e))),
    }
}

// ═══════════════════════════════════════════════════════════════
// POST /targets/{id}/pin
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_pin(
    path:     web::Path<String>,
    body:     web::Json<ToggleRequest>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    let (mut state, evidence_id, ev_number, _, _) =
        match load_target_state(&conn, &target_id, &user_id) {
            Ok(v)  => v,
            Err(e) => return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("{}", e))),
        };

    // Toggle or use explicit body value
    let new_val = body.active.unwrap_or(!state.pinned);
    state.pinned = new_val;

    if let Err(e) = update_flag_col(&conn, &target_id, &user_id, "is_pinned", new_val) {
        return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Update failed: {}", e)));
    }

    let action_word = if new_val { "pinned as HOT / WANTED" } else { "unpinned" };
    let notif_title = format!("Target {}", if new_val { "Pinned as HOT" } else { "Unpinned" });
    let notif_msg   = format!(
        "Target from evidence {} has been {}.",
        ev_number, action_word
    );

    notify_target_action(&database, &user_id, "target_pin", &notif_title, &notif_msg, &evidence_id, None).await;
    audit_target_action(&database, &user_id, "target_pin", &target_id,
        &format!("Target {} {} for evidence {}", target_id, action_word, ev_number)).await;

    println!("📌 TARGET_PIN: target={} user={} active={}", target_id, user_id, new_val);

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "target_id": target_id,
        "pinned":    new_val,
        "message":   notif_title,
    })))
}

// ═══════════════════════════════════════════════════════════════
// POST /targets/{id}/poi
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_poi(
    path:     web::Path<String>,
    body:     web::Json<ToggleRequest>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    let (mut state, evidence_id, ev_number, category, _) =
        match load_target_state(&conn, &target_id, &user_id) {
            Ok(v)  => v,
            Err(e) => return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("{}", e))),
        };

    let new_val = body.active.unwrap_or(!state.poi);
    state.poi = new_val;

    if let Err(e) = update_flag_col(&conn, &target_id, &user_id, "is_poi", new_val) {
        return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Update failed: {}", e)));
    }

    let notif_title = if new_val { "Target Marked as POI".to_string() }
                      else       { "POI Status Removed".to_string() };
    let notif_msg   = format!(
        "{} target from evidence {} has been {}.",
        category, ev_number,
        if new_val { "marked as a Person of Interest" } else { "removed from POI list" }
    );

    notify_target_action(&database, &user_id, "target_poi", &notif_title, &notif_msg, &evidence_id, None).await;
    audit_target_action(&database, &user_id, "target_poi", &target_id,
        &format!("Target {} POI={} for evidence {}", target_id, new_val, ev_number)).await;

    println!("🎯 TARGET_POI: target={} user={} active={}", target_id, user_id, new_val);

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "target_id": target_id,
        "poi":       new_val,
        "message":   notif_title,
    })))
}

// ═══════════════════════════════════════════════════════════════
// POST /targets/{id}/watchlist
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_watchlist(
    path:     web::Path<String>,
    body:     web::Json<ToggleRequest>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    let (mut state, evidence_id, ev_number, _, _) =
        match load_target_state(&conn, &target_id, &user_id) {
            Ok(v)  => v,
            Err(e) => return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("{}", e))),
        };

    let new_val = body.active.unwrap_or(!state.watchlist);
    state.watchlist = new_val;

    if let Err(e) = update_flag_col(&conn, &target_id, &user_id, "is_watchlist", new_val) {
        return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Update failed: {}", e)));
    }

    let notif_title = if new_val { "Target Added to Watchlist".to_string() }
                      else       { "Target Removed from Watchlist".to_string() };
    let notif_msg   = format!(
        "Target from evidence {} has been {}.",
        ev_number,
        if new_val { "added to your watchlist" } else { "removed from watchlist" }
    );

    notify_target_action(&database, &user_id, "target_watchlist", &notif_title, &notif_msg, &evidence_id, None).await;
    audit_target_action(&database, &user_id, "target_watchlist", &target_id,
        &format!("Target {} watchlist={} for evidence {}", target_id, new_val, ev_number)).await;

    println!("👁 TARGET_WATCHLIST: target={} user={} active={}", target_id, user_id, new_val);

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "target_id": target_id,
        "watchlist": new_val,
        "message":   notif_title,
    })))
}

// ═══════════════════════════════════════════════════════════════
// POST /targets/{id}/flag
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_flag(
    path:     web::Path<String>,
    body:     web::Json<ToggleRequest>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    let (mut state, evidence_id, ev_number, _, _) =
        match load_target_state(&conn, &target_id, &user_id) {
            Ok(v)  => v,
            Err(e) => return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("{}", e))),
        };

    let new_val = body.active.unwrap_or(!state.flagged);
    state.flagged = new_val;

    if let Err(e) = update_flag_col(&conn, &target_id, &user_id, "is_flagged", new_val) {
        return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Update failed: {}", e)));
    }

    let notif_title = if new_val { "Target Flagged for Review".to_string() }
                      else       { "Target Flag Cleared".to_string() };
    let notif_msg   = format!(
        "Target from evidence {} has been {}.",
        ev_number,
        if new_val { "flagged for supervisor review" } else { "unflagged" }
    );

    notify_target_action(&database, &user_id, "target_flag", &notif_title, &notif_msg, &evidence_id, None).await;
    audit_target_action(&database, &user_id, "target_flag", &target_id,
        &format!("Target {} flagged={} for evidence {}", target_id, new_val, ev_number)).await;

    println!("🚩 TARGET_FLAG: target={} user={} active={}", target_id, user_id, new_val);

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "target_id": target_id,
        "flagged":   new_val,
        "message":   notif_title,
    })))
}

// ═══════════════════════════════════════════════════════════════
// DELETE /targets/{id}/takedown
// Marks is_takedown = 1, logs, notifies, then REMOVES the target row.
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_takedown(
    path:     web::Path<String>,
    _body:    web::Json<serde_json::Value>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    // Load state first (also upserts the flag row)
    let (_, evidence_id, ev_number, category, description) =
        match load_target_state(&conn, &target_id, &user_id) {
            Ok(v)  => v,
            Err(e) => return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("{}", e))),
        };

    // Mark takedown in flags table (persists the tombstone even after target delete)
    if let Err(e) = update_flag_col(&conn, &target_id, &user_id, "is_takedown", true) {
        return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Flag update failed: {}", e)));
    }

    // Hard-delete the target from the targets table
    let deleted = conn.execute(
        "DELETE FROM targets WHERE id = ?1",
        params![target_id],
    ).unwrap_or(0);

    let notif_msg = format!(
        "Takedown order issued for {} target from evidence {}. Description: {}. Record permanently removed.",
        category, ev_number, description
    );

    notify_target_action(&database, &user_id, "target_takedown",
        "Takedown Order Issued", &notif_msg, &evidence_id, None).await;
    audit_target_action(&database, &user_id, "target_takedown", &target_id,
        &format!(
            "TAKEDOWN: target={} evidence={} category={} deleted_rows={}",
            target_id, ev_number, category, deleted
        )).await;

    println!("🚫 TARGET_TAKEDOWN: target={} user={} evidence={} deleted={}",
        target_id, user_id, ev_number, deleted);

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "target_id": target_id,
        "takedown":  true,
        "deleted":   deleted > 0,
        "message":   "Takedown order issued. Target permanently removed.",
    })))
}

// ═══════════════════════════════════════════════════════════════
// POST /targets/{id}/notes
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_notes(
    path:     web::Path<String>,
    body:     web::Json<NotesRequest>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };
    let notes     = body.notes.trim().to_string();

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    // Ensure row exists
    let (_, evidence_id, ev_number, _, _) =
        match load_target_state(&conn, &target_id, &user_id) {
            Ok(v)  => v,
            Err(e) => return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("{}", e))),
        };

    conn.execute(
        "UPDATE target_flags SET notes = ?1, updated_at = ?2 WHERE target_id = ?3 AND user_id = ?4",
        params![notes, Utc::now().timestamp(), target_id, user_id],
    ).unwrap_or(0);

    let notif_msg = format!("Investigator notes updated for target on evidence {}.", ev_number);
    notify_target_action(&database, &user_id, "target_notes",
        "Case Notes Updated", &notif_msg, &evidence_id, None).await;
    audit_target_action(&database, &user_id, "target_notes", &target_id,
        &format!("Notes saved for target {} (evidence {})", target_id, ev_number)).await;

    println!("📝 TARGET_NOTES: target={} user={}", target_id, user_id);

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "target_id": target_id,
        "message":   "Case notes saved",
    })))
}

// ═══════════════════════════════════════════════════════════════
// POST /targets/{id}/link-case
// Appends a case ref to linked_case_refs (JSON array) in the flags row.
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_link_case(
    path:     web::Path<String>,
    body:     web::Json<LinkCaseRequest>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };
    let case_ref  = body.case_ref.trim().to_string();

    if case_ref.is_empty() {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("case_ref cannot be empty"));
    }

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    let (mut state, evidence_id, ev_number, _, _) =
        match load_target_state(&conn, &target_id, &user_id) {
            Ok(v)  => v,
            Err(e) => return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("{}", e))),
        };

    // Append case ref if not already present
    if !state.linked_cases.contains(&case_ref) {
        state.linked_cases.push(case_ref.clone());
    }
    let new_json = serde_json::to_string(&state.linked_cases).unwrap_or_else(|_| "[]".into());

    conn.execute(
        "UPDATE target_flags SET linked_case_refs = ?1, updated_at = ?2 WHERE target_id = ?3 AND user_id = ?4",
        params![new_json, Utc::now().timestamp(), target_id, user_id],
    ).unwrap_or(0);

    let notif_msg = format!(
        "Target from evidence {} has been linked to case reference #{}.",
        ev_number, case_ref
    );
    notify_target_action(&database, &user_id, "target_link_case",
        "Target Linked to Case", &notif_msg, &evidence_id, None).await;
    audit_target_action(&database, &user_id, "target_link_case", &target_id,
        &format!("Target {} linked to case #{} (evidence {})", target_id, case_ref, ev_number)).await;

    println!("🔗 TARGET_LINK_CASE: target={} case_ref={} user={}", target_id, case_ref, user_id);

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "target_id":   target_id,
        "case_ref":    case_ref,
        "total_cases": state.linked_cases.len(),
        "cases":       state.linked_cases,
        "message":     format!("Target linked to case #{}", case_ref),
    })))
}

// ═══════════════════════════════════════════════════════════════
// GET /targets/{id}/linked-cases
// Returns linked cases list for the card chip count AND modal section.
// ═══════════════════════════════════════════════════════════════

pub async fn api_target_linked_cases(
    path:     web::Path<String>,
    session:  Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let target_id = path.into_inner();
    let user_id   = match require_user_id(&session) { Ok(v) => v, Err(r) => return r };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    let (state, _, _, _, _) = match load_target_state(&conn, &target_id, &user_id) {
        Ok(v)  => v,
        Err(_) => {
            // Target may not exist yet — return empty
            return HttpResponse::Ok().json(ApiResponse::success(json!({
                "cases": [],
                "count": 0,
            })));
        }
    };

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "cases": state.linked_cases,
        "count": state.linked_cases.len(),
    })))
}

// ═══════════════════════════════════════════════════════════════
// ROUTE CONFIG — add .configure(target_routes::config) in main.rs
// ═══════════════════════════════════════════════════════════════

pub fn config(cfg: &mut web::ServiceConfig) {
    // Bootstrap table the first time routes are registered.
    // We need a reference to the Database — done lazily in each handler instead,
    // so no Data<Database> is available here at config time.
    // Table creation is idempotent and fast; it's guarded by IF NOT EXISTS.

    cfg
        // State loader (GET — called by openTargetProfile on open)
        .route("/targets/{id}/state",        web::get().to(api_get_target_state))
        // Toggle actions (POST)
        .route("/targets/{id}/pin",          web::post().to(api_target_pin))
        .route("/targets/{id}/poi",          web::post().to(api_target_poi))
        .route("/targets/{id}/watchlist",    web::post().to(api_target_watchlist))
        .route("/targets/{id}/flag",         web::post().to(api_target_flag))
        // Notes (POST)
        .route("/targets/{id}/notes",        web::post().to(api_target_notes))
        // Link case (POST)
        .route("/targets/{id}/link-case",    web::post().to(api_target_link_case))
        // Linked cases list (GET) — used by card chip AND modal section
        .route("/targets/{id}/linked-cases", web::get().to(api_target_linked_cases))
        // Takedown (DELETE)
        .route("/targets/{id}/takedown",     web::delete().to(api_target_takedown));
}

// Call this ONCE from main.rs after the Database is initialised:
//   target_routes::init_table(&database.get_ref());
pub fn init_table(db: &Database) {
    ensure_target_flags_table(db);
    println!("✅ target_flags table ready");
}