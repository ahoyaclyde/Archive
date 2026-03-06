// src/intelligence_routes.rs
//
// ══════════════════════════════════════════════════════════════════════════
//  INTELLIGENCE SUBJECT SYSTEM
//
//  Completely separate from target_routes.rs which handles per-user
//  toggle actions on uploaded target photos (pin/poi/watchlist/flag).
//
//  This module manages NAMED SUBJECTS — intelligence entities that can
//  appear across many evidence records — and the FLAGS that link them.
//
//  Tables (created idempotently on startup):
//
//    subjects      — Global named-entity registry (person, vehicle, object …)
//                    One row per real-world entity, shared across all users.
//
//    intel_flags   — Per-evidence intelligence flags that link an evidence
//                    record to a subject (and optionally to an uploaded
//                    target photo from target_routes.rs territory).
//                    Many flags can reference the same subject → automatic
//                    cross-case correlation.
//
//  Endpoints:
//    GET    /api/subjects/search                     — live search w/ case count
//    GET    /api/subjects/:id/appearances            — all cases for one subject
//    POST   /api/evidence/:id/flag-target            — create flag
//    GET    /api/evidence/:id/flag-targets           — all flags for evidence
//    DELETE /api/evidence/:eid/flag-target/:fid      — remove one flag
//
//  REGISTER in main.rs:
//    mod intelligence_routes;
//    // inside HttpServer::new:
//    .configure(intelligence_routes::config)
//
//  REGISTER in lib.rs:
//    pub mod intelligence_routes;
//
//  Call once from main.rs after Database::new:
//    intelligence_routes::init_tables(database.get_ref());
// ══════════════════════════════════════════════════════════════════════════

use actix_session::Session;
use actix_web::{web, HttpResponse};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::database::Database;
use crate::models::ApiResponse;

// ═══════════════════════════════════════════════════════════════════════════
//  MODELS
// ═══════════════════════════════════════════════════════════════════════════

/// A named real-world entity in the global subject registry.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subject {
    pub id:                   String,
    pub name:                 String,
    pub description:          Option<String>,
    pub category:             String,
    pub last_known_location:  Option<String>,
    pub age:                  Option<i32>,
    pub physical_description: Option<String>,
    pub charges:              Option<String>,
    pub date_missing:         Option<String>,
    pub created_by:           String,
    pub created_at:           String,
    /// How many distinct evidence records this subject appears in (computed).
    pub appearance_count:     i64,
    /// Which flag types this subject carries across all evidence (computed).
    pub flag_types:           Vec<String>,
}

/// A single intelligence flag linking one evidence record to one subject.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IntelFlag {
    pub id:                  String,
    pub evidence_id:         String,
    pub subject_id:          Option<String>,
    /// Optional link to an uploaded target photo (target_routes.rs territory).
    pub target_photo_id:     Option<String>,
    pub flag_type:           String,   // poi | watchlist | wanted | missing
    pub confidence:          i32,
    pub notes:               Option<String>,
    pub last_known_location: Option<String>,
    pub created_by:          String,
    pub created_at:          String,
    // Joined from subjects
    pub subject_name:        Option<String>,
    pub subject_category:    Option<String>,
    pub appearance_count:    i64,
}

// ─── Request bodies ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SubjectSearchQuery {
    pub q:     Option<String>,
    pub limit: Option<i64>,
}

/// POST /api/evidence/:id/flag-target
///
/// Caller provides EITHER `subject_id` (link existing subject)
/// OR `name` + fields (create new subject on the fly).
#[derive(Debug, Deserialize)]
pub struct FlagTargetRequest {
    // Subject linkage
    pub subject_id:          Option<String>,
    pub name:                Option<String>,
    pub description:         Option<String>,
    pub category:            Option<String>,
    pub physical_description: Option<String>,
    // Flag metadata
    pub flag_type:           String,
    pub confidence:          Option<i32>,
    pub notes:               Option<String>,
    pub last_known_location: Option<String>,
    // Optional link to an uploaded target photo
    pub target_photo_id:     Option<String>,
    // Type-specific extras
    pub age:                 Option<i32>,
    pub date_missing:        Option<String>,
    pub charges:             Option<String>,
}

// ═══════════════════════════════════════════════════════════════════════════
//  TABLE BOOTSTRAP
// ═══════════════════════════════════════════════════════════════════════════

pub fn init_tables(db: &Database) {
    let conn = match db.pool.get() {
        Ok(c)  => c,
        Err(e) => { eprintln!("❌ INTEL: DB pool error: {}", e); return; }
    };

    let sql = "
        -- Global named-entity registry
        CREATE TABLE IF NOT EXISTS subjects (
            id                   TEXT PRIMARY KEY,
            name                 TEXT NOT NULL,
            description          TEXT,
            category             TEXT NOT NULL DEFAULT 'person',
            last_known_location  TEXT,
            age                  INTEGER,
            physical_description TEXT,
            charges              TEXT,
            date_missing         TEXT,
            created_by           TEXT NOT NULL,
            created_at           TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at           TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_subjects_name
            ON subjects (name COLLATE NOCASE);

        -- Per-evidence intelligence flags
        -- NOTE: table is named intel_flags to avoid collision with
        --       target_routes.rs which owns the target_flags table.
        CREATE TABLE IF NOT EXISTS intel_flags (
            id                  TEXT PRIMARY KEY,
            evidence_id         TEXT NOT NULL,
            subject_id          TEXT,
            target_photo_id     TEXT,   -- optional FK to target_routes' targets table
            flag_type           TEXT NOT NULL,
            confidence          INTEGER NOT NULL DEFAULT 70,
            notes               TEXT,
            last_known_location TEXT,
            created_by          TEXT NOT NULL,
            created_at          TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (subject_id) REFERENCES subjects(id)
        );
        CREATE INDEX IF NOT EXISTS idx_intel_flags_evidence
            ON intel_flags (evidence_id);
        CREATE INDEX IF NOT EXISTS idx_intel_flags_subject
            ON intel_flags (subject_id);
        CREATE INDEX IF NOT EXISTS idx_intel_flags_type
            ON intel_flags (flag_type);
    ";

    match conn.execute_batch(sql) {
        Ok(_)  => println!("✅ INTEL: subjects + intel_flags tables ready"),
        Err(e) => eprintln!("❌ INTEL: Table creation failed: {}", e),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  INTERNAL HELPERS
// ═══════════════════════════════════════════════════════════════════════════

fn require_user(session: &Session) -> Result<String, HttpResponse> {
    match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => Ok(id),
        None => Err(HttpResponse::Unauthorized()
            .json(ApiResponse::<()>::error("Not authenticated"))),
    }
}

/// Fire a notification for an intelligence action (mirrors target_routes pattern).
async fn notify_intel_action(
    db:          &Database,
    user_id:     &str,
    notif_type:  &str,
    title:       &str,
    message:     &str,
    evidence_id: &str,
) {
    let eid = if evidence_id.is_empty() { None } else { Some(evidence_id) };
    let _ = db.create_notification(user_id, notif_type, title, message, eid, None, None).await;
}

/// Write an audit log entry for an intelligence action.
async fn audit_intel_action(
    db:        &Database,
    user_id:   &str,
    action:    &str,
    entity_id: &str,
    details:   &str,
) {
    let _ = db.log_audit(Some(user_id), action, "intel_flag", Some(entity_id), details, None).await;
}

// ═══════════════════════════════════════════════════════════════════════════
//  HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

// ── GET /api/subjects/search ──────────────────────────────────────────────
//
//  Live search — returns subjects whose name contains `q`, each enriched
//  with their cross-case appearance count and flag types.
pub async fn api_subject_search(
    session:  Session,
    query:    web::Query<SubjectSearchQuery>,
    database: web::Data<Database>,
) -> HttpResponse {
    if let Err(r) = require_user(&session) { return r; }

    let q     = query.q.clone().unwrap_or_default();
    let limit = query.limit.unwrap_or(8).min(20);

    let db = database.clone();
    let result = web::block(move || -> Result<Vec<serde_json::Value>, String> {
        let conn    = db.pool.get().map_err(|e| e.to_string())?;
        let pattern = format!("%{}%", q.trim());

        let mut stmt = conn.prepare("
            SELECT
                s.id,
                s.name,
                s.description,
                s.category,
                s.last_known_location,
                s.age,
                s.physical_description,
                s.charges,
                s.date_missing,
                s.created_at,
                COUNT(DISTINCT f.evidence_id)        AS appearance_count,
                GROUP_CONCAT(DISTINCT f.flag_type)   AS flag_types
            FROM subjects s
            LEFT JOIN intel_flags f ON f.subject_id = s.id
            WHERE s.name LIKE ?1 COLLATE NOCASE
            GROUP BY s.id
            ORDER BY appearance_count DESC, s.name ASC
            LIMIT ?2
        ").map_err(|e| e.to_string())?;

        let rows = stmt.query_map(
            params![pattern, limit],
            |row| {
                let raw: Option<String> = row.get(11)?;
                let flag_types: Vec<String> = raw
                    .unwrap_or_default()
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();

                Ok(json!({
                    "id":                   row.get::<_, String>(0)?,
                    "name":                 row.get::<_, String>(1)?,
                    "description":          row.get::<_, Option<String>>(2)?,
                    "category":             row.get::<_, String>(3)?,
                    "last_known_location":  row.get::<_, Option<String>>(4)?,
                    "age":                  row.get::<_, Option<i32>>(5)?,
                    "physical_description": row.get::<_, Option<String>>(6)?,
                    "charges":              row.get::<_, Option<String>>(7)?,
                    "date_missing":         row.get::<_, Option<String>>(8)?,
                    "created_at":           row.get::<_, String>(9)?,
                    "appearance_count":     row.get::<_, i64>(10)?,
                    "flag_types":           flag_types,
                }))
            },
        ).map_err(|e| e.to_string())?;

        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
    }).await;

    match result {
        Ok(Ok(subjects)) => {
            println!("🔍 INTEL SEARCH: '{}' → {} results", query.q.as_deref().unwrap_or(""), subjects.len());
            HttpResponse::Ok().json(ApiResponse::success(subjects))
        }
        Ok(Err(e)) => {
            println!("❌ INTEL SEARCH: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e))
        }
        Err(e) => {
            println!("❌ INTEL SEARCH: block: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Search failed"))
        }
    }
}

// ── GET /api/subjects/:id/appearances ────────────────────────────────────
//
//  All evidence cases where this subject has been flagged.
pub async fn api_subject_appearances(
    session:  Session,
    path:     web::Path<String>,
    database: web::Data<Database>,
) -> HttpResponse {
    if let Err(r) = require_user(&session) { return r; }

    let subject_id = path.into_inner();
    let db = database.clone();

    let result = web::block(move || -> Result<serde_json::Value, String> {
        let conn = db.pool.get().map_err(|e| e.to_string())?;

        // Subject header
        let subject_opt: Option<serde_json::Value> = conn.query_row(
            "SELECT id, name, description, category, last_known_location, age,
                    physical_description, charges, date_missing, created_at
             FROM subjects WHERE id = ?1",
            [&subject_id],
            |row| Ok(json!({
                "id":                   row.get::<_, String>(0)?,
                "name":                 row.get::<_, String>(1)?,
                "description":          row.get::<_, Option<String>>(2)?,
                "category":             row.get::<_, String>(3)?,
                "last_known_location":  row.get::<_, Option<String>>(4)?,
                "age":                  row.get::<_, Option<i32>>(5)?,
                "physical_description": row.get::<_, Option<String>>(6)?,
                "charges":              row.get::<_, Option<String>>(7)?,
                "date_missing":         row.get::<_, Option<String>>(8)?,
                "created_at":           row.get::<_, String>(9)?,
            })),
        ).optional().map_err(|e: rusqlite::Error| e.to_string())?;

        let subject = subject_opt.ok_or_else(|| "Subject not found".to_string())?;

        // All evidence appearances — joined with evidence for context
        let mut stmt = conn.prepare("
            SELECT
                f.id              AS flag_id,
                f.evidence_id,
                f.flag_type,
                f.confidence,
                f.notes,
                f.last_known_location,
                f.target_photo_id,
                f.created_at      AS flagged_at,
                e.evidence_number,
                e.title,
                e.status,
                e.county,
                e.incident_time
            FROM intel_flags f
            LEFT JOIN evidence e ON e.id = f.evidence_id
            WHERE f.subject_id = ?1
            ORDER BY f.created_at DESC
        ").map_err(|e| e.to_string())?;

        let appearances: Vec<serde_json::Value> = stmt
            .query_map([&subject_id], |row| Ok(json!({
                "flag_id":             row.get::<_, String>(0)?,
                "evidence_id":         row.get::<_, String>(1)?,
                "flag_type":           row.get::<_, String>(2)?,
                "confidence":          row.get::<_, i32>(3)?,
                "notes":               row.get::<_, Option<String>>(4)?,
                "last_known_location": row.get::<_, Option<String>>(5)?,
                "target_photo_id":     row.get::<_, Option<String>>(6)?,
                "flagged_at":          row.get::<_, String>(7)?,
                "evidence_number":     row.get::<_, Option<String>>(8)?,
                "title":               row.get::<_, Option<String>>(9)?,
                "status":              row.get::<_, Option<String>>(10)?,
                "county":              row.get::<_, Option<String>>(11)?,
                "incident_time":       row.get::<_, Option<String>>(12)?,
            })))
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        let total = appearances.len();
        Ok(json!({
            "subject":     subject,
            "appearances": appearances,
            "total_cases": total,
        }))
    }).await;

    match result {
        Ok(Ok(data))  => {
            println!("👤 INTEL APPEARANCES: {} cases", data["total_cases"]);
            HttpResponse::Ok().json(ApiResponse::success(data))
        }
        Ok(Err(ref e)) if e == "Subject not found" =>
            HttpResponse::NotFound().json(ApiResponse::<()>::error("Subject not found")),
        Ok(Err(e)) => {
            println!("❌ INTEL APPEARANCES: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e))
        }
        Err(e) => {
            println!("❌ INTEL APPEARANCES: block: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed"))
        }
    }
}

// ── POST /api/evidence/:id/flag-target ───────────────────────────────────
//
//  Creates an intelligence flag for this evidence.
//  If `subject_id` supplied → links existing subject.
//  If only `name` supplied  → creates a new subject first.
//  Fires notification + audit log, matching target_routes.rs pattern.
pub async fn api_create_intel_flag(
    session:  Session,
    path:     web::Path<String>,
    body:     web::Json<FlagTargetRequest>,
    database: web::Data<Database>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = match require_user(&session) { Ok(id) => id, Err(r) => return r };
    let payload = body.into_inner();

    // Validate
    if payload.subject_id.is_none() && payload.name.as_deref().unwrap_or("").trim().is_empty() {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("Provide subject_id or a name"));
    }
    let valid_types = ["poi", "watchlist", "wanted", "missing"];
    if !valid_types.contains(&payload.flag_type.to_lowercase().as_str()) {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("flag_type must be: poi, watchlist, wanted, or missing"));
    }

    let db  = database.clone();
    let uid = user_id.clone();
    let eid = evidence_id.clone();

    let result = web::block(move || -> Result<serde_json::Value, String> {
        let conn = db.pool.get().map_err(|e| e.to_string())?;

        // ── Resolve subject ────────────────────────────────────────────────
        let subject_id: String =
            if let Some(sid) = payload.subject_id.filter(|s| !s.is_empty()) {
                // Verify exists
                let ok: bool = conn.query_row(
                    "SELECT 1 FROM subjects WHERE id = ?1",
                    [&sid], |_| Ok(true),
                ).optional().map_err(|e: rusqlite::Error| e.to_string())?.unwrap_or(false);
                if !ok { return Err(format!("Subject {} not found", sid)); }

                // Optionally update enrichment fields the caller supplied
                conn.execute(
                    "UPDATE subjects SET
                         charges             = COALESCE(?1, charges),
                         date_missing        = COALESCE(?2, date_missing),
                         last_known_location = COALESCE(?3, last_known_location),
                         updated_at          = datetime('now')
                     WHERE id = ?4",
                    params![payload.charges, payload.date_missing, payload.last_known_location, sid],
                ).map_err(|e| e.to_string())?;

                sid
            } else {
                // Create new subject
                let new_id = Uuid::new_v4().to_string();
                let name   = payload.name.as_deref().unwrap_or("").trim().to_string();

                conn.execute(
                    "INSERT INTO subjects
                         (id, name, description, category, last_known_location,
                          age, physical_description, charges, date_missing, created_by)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        new_id,
                        name,
                        payload.description,
                        payload.category.as_deref().unwrap_or("person"),
                        payload.last_known_location,
                        payload.age,
                        payload.physical_description,
                        payload.charges,
                        payload.date_missing,
                        uid,
                    ],
                ).map_err(|e| e.to_string())?;

                println!("👤 INTEL: new subject '{}' id={}", name, new_id);
                new_id
            };

        // ── Create the intel flag ──────────────────────────────────────────
        let flag_id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO intel_flags
                 (id, evidence_id, subject_id, target_photo_id, flag_type,
                  confidence, notes, last_known_location, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                flag_id,
                eid,
                subject_id,
                payload.target_photo_id,
                payload.flag_type.to_lowercase(),
                payload.confidence.unwrap_or(70),
                payload.notes,
                payload.last_known_location,
                uid,
            ],
        ).map_err(|e| e.to_string())?;

        // ── Return enriched flag ───────────────────────────────────────────
        let flag = conn.query_row(
            "SELECT
                 f.id, f.evidence_id, f.subject_id, f.target_photo_id,
                 f.flag_type, f.confidence, f.notes, f.last_known_location,
                 f.created_at,
                 s.name                AS subject_name,
                 s.category            AS subject_category,
                 (SELECT COUNT(DISTINCT f2.evidence_id)
                  FROM intel_flags f2 WHERE f2.subject_id = s.id) AS appearance_count
             FROM intel_flags f
             LEFT JOIN subjects s ON s.id = f.subject_id
             WHERE f.id = ?1",
            [&flag_id],
            |row| Ok(json!({
                "id":                  row.get::<_, String>(0)?,
                "evidence_id":         row.get::<_, String>(1)?,
                "subject_id":          row.get::<_, Option<String>>(2)?,
                "target_photo_id":     row.get::<_, Option<String>>(3)?,
                "flag_type":           row.get::<_, String>(4)?,
                "confidence":          row.get::<_, i32>(5)?,
                "notes":               row.get::<_, Option<String>>(6)?,
                "last_known_location": row.get::<_, Option<String>>(7)?,
                "created_at":          row.get::<_, String>(8)?,
                "name":                row.get::<_, Option<String>>(9)?,
                "category":            row.get::<_, Option<String>>(10)?,
                "appearance_count":    row.get::<_, i64>(11)?,
            })),
        ).map_err(|e| e.to_string())?;

        println!("🎯 INTEL FLAG: created id={} type={} subject={}", flag_id, payload.flag_type, subject_id);
        Ok(flag)
    }).await;

    match result {
        Ok(Ok(flag)) => {
            // Notification + audit (mirrors target_routes.rs pattern)
            let subject_name = flag["name"].as_str().unwrap_or("Unknown").to_string();
            let flag_type    = flag["flag_type"].as_str().unwrap_or("").to_string();
            let cases        = flag["appearance_count"].as_i64().unwrap_or(1);

            let notif_title = format!("Intelligence Flag: {}", flag_type.to_uppercase());
            let notif_msg   = format!(
                "{} flagged as {} on evidence {}. Subject appears in {} case(s).",
                subject_name, flag_type, evidence_id, cases
            );

            notify_intel_action(&database, &user_id, "intel_flag_created",
                &notif_title, &notif_msg, &evidence_id).await;
            audit_intel_action(&database, &user_id, "intel_flag_created",
                flag["id"].as_str().unwrap_or(""),
                &format!("Flag type={} subject={} evidence={} cases={}", flag_type, subject_name, evidence_id, cases)).await;

            HttpResponse::Ok().json(ApiResponse::success(flag))
        }
        Ok(Err(e)) => {
            println!("❌ INTEL FLAG: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e))
        }
        Err(e) => {
            println!("❌ INTEL FLAG: block: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to create flag"))
        }
    }
}

// ── GET /api/evidence/:id/flag-targets ───────────────────────────────────
//
//  Returns all intelligence flags for one evidence record, each enriched
//  with subject metadata and cross-case appearance count.
pub async fn api_get_intel_flags(
    session:  Session,
    path:     web::Path<String>,
    database: web::Data<Database>,
) -> HttpResponse {
    if let Err(r) = require_user(&session) { return r; }

    let evidence_id = path.into_inner();
    let db = database.clone();

    let result = web::block(move || -> Result<Vec<serde_json::Value>, String> {
        let conn = db.pool.get().map_err(|e| e.to_string())?;

        let mut stmt = conn.prepare("
            SELECT
                f.id,
                f.evidence_id,
                f.subject_id,
                f.target_photo_id,
                f.flag_type,
                f.confidence,
                f.notes,
                f.last_known_location,
                f.created_at,
                s.name                AS subject_name,
                s.category            AS subject_category,
                s.physical_description,
                s.age,
                s.charges,
                s.date_missing,
                (SELECT COUNT(DISTINCT f2.evidence_id)
                 FROM intel_flags f2 WHERE f2.subject_id = s.id) AS appearance_count
            FROM intel_flags f
            LEFT JOIN subjects s ON s.id = f.subject_id
            WHERE f.evidence_id = ?1
            ORDER BY f.created_at DESC
        ").map_err(|e| e.to_string())?;

        let flags = stmt.query_map([&evidence_id], |row| {
            Ok(json!({
                "id":                  row.get::<_, String>(0)?,
                "evidence_id":         row.get::<_, String>(1)?,
                "subject_id":          row.get::<_, Option<String>>(2)?,
                "target_photo_id":     row.get::<_, Option<String>>(3)?,
                "flag_type":           row.get::<_, String>(4)?,
                "confidence":          row.get::<_, i32>(5)?,
                "notes":               row.get::<_, Option<String>>(6)?,
                "last_known_location": row.get::<_, Option<String>>(7)?,
                "created_at":          row.get::<_, String>(8)?,
                "name":                row.get::<_, Option<String>>(9)?,
                "category":            row.get::<_, Option<String>>(10)?,
                "physical_description":row.get::<_, Option<String>>(11)?,
                "age":                 row.get::<_, Option<i32>>(12)?,
                "charges":             row.get::<_, Option<String>>(13)?,
                "date_missing":        row.get::<_, Option<String>>(14)?,
                "appearance_count":    row.get::<_, i64>(15)?,
            }))
        }).map_err(|e| e.to_string())?
          .collect::<Result<Vec<_>, _>>()
          .map_err(|e| e.to_string())?;

        println!("🎯 INTEL GET FLAGS: evidence={} → {} flags", evidence_id, flags.len());
        Ok(flags)
    }).await;

    match result {
        Ok(Ok(flags)) => HttpResponse::Ok().json(ApiResponse::success(flags)),
        Ok(Err(e))    => {
            println!("❌ INTEL GET FLAGS: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&e))
        }
        Err(e) => {
            println!("❌ INTEL GET FLAGS: block: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to load flags"))
        }
    }
}

// ── DELETE /api/evidence/:eid/flag-target/:fid ───────────────────────────
pub async fn api_delete_intel_flag(
    session:  Session,
    path:     web::Path<(String, String)>,
    database: web::Data<Database>,
) -> HttpResponse {
    let user_id = match require_user(&session) { Ok(id) => id, Err(r) => return r };
    let (evidence_id, flag_id) = path.into_inner();
    let db  = database.clone();
    let uid = user_id.clone();
    let fid = flag_id.clone();

    let result = web::block(move || -> Result<(), String> {
        let conn = db.pool.get().map_err(|e| e.to_string())?;

        let rows = conn.execute(
            "DELETE FROM intel_flags
             WHERE id = ?1 AND evidence_id = ?2 AND created_by = ?3",
            params![fid, evidence_id, uid],
        ).map_err(|e| e.to_string())?;

        if rows == 0 {
            return Err("Flag not found or permission denied".to_string());
        }
        println!("🗑️  INTEL DELETE FLAG: id={}", fid);
        Ok(())
    }).await;

    match result {
        Ok(Ok(())) => {
            audit_intel_action(&database, &user_id, "intel_flag_deleted",
                &flag_id, &format!("Flag {} removed from evidence", flag_id)).await;
            HttpResponse::Ok().json(ApiResponse::success(json!({"deleted": true})))
        }
        Ok(Err(e)) => {
            println!("❌ INTEL DELETE: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&e))
        }
        Err(e) => {
            println!("❌ INTEL DELETE: block: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Delete failed"))
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  ROUTE CONFIG
// ═══════════════════════════════════════════════════════════════════════════

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        // Global subject registry
        .route("/api/subjects/search",
               web::get().to(api_subject_search))
        .route("/api/subjects/{id}/appearances",
               web::get().to(api_subject_appearances))

        // Per-evidence intelligence flags
        .route("/api/evidence/{id}/flag-target",
               web::post().to(api_create_intel_flag))
        .route("/api/evidence/{id}/flag-targets",
               web::get().to(api_get_intel_flags))
        .route("/api/evidence/{eid}/flag-target/{fid}",
               web::delete().to(api_delete_intel_flag));
}