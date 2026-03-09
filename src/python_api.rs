// src/python_api.rs
//
// ═══════════════════════════════════════════════════════════════════════════
// FLUG Evidence Platform — Python System API  (v1)
// ═══════════════════════════════════════════════════════════════════════════
//
// All routes live under /api/v1/ and require the header:
//     X-API-Key: <value of PYTHON_API_KEY env var>
// except GET /api/v1/health and POST /api/v1/auth/login.
//
// ── Wire up in main.rs ────────────────────────────────────────────────────
//   mod python_api;
//   // inside HttpServer::new closure:
//   .configure(python_api::config)
//
// ── Required env vars ─────────────────────────────────────────────────────
//   PYTHON_API_KEY   — any string ≥ 32 chars; generate: openssl rand -hex 32
//
// ── Route index ──────────────────────────────────────────────────────────
//
//  HEALTH
//    GET  /api/v1/health
//
//  AUTH
//    POST /api/v1/auth/login              { email, password }
//      → 200 { success, api_key, user: { id, email, account_type, phone_number,
//               county, wallet_address, wallet_chain, public_key, … } }
//
//  EVIDENCE — read
//    GET  /api/v1/evidence
//      ?page,limit,status,emergency_level,incident_type,
//       county,constituency,ward,country,query,sort_by,
//       date_from,date_to,lat,lng,radius_km
//    GET  /api/v1/evidence/{id}
//
//  EVIDENCE — location compound query
//    GET  /api/v1/evidence/by-location
//      ?county,constituency,ward,country,incident_type,
//       lat,lng,radius_km,mode,include_encodings,page,limit
//      mode=1 → evidence JSON only (default)
//      mode=2 → evidence + all targets (raw image URLs + encoding bytes)
//
//  EVIDENCE — write
//    POST /api/v1/evidence/{id}/update
//    POST /api/v1/evidence/{id}/status    { status, reason? }
//
//  TARGETS — read
//    GET  /api/v1/targets/by-evidence/{evidence_id}  ?include_encodings
//    GET  /api/v1/targets/by-location
//          ?county,constituency,ward,country,incident_type,category,
//           include_encodings,lat,lng,radius_km,page,limit
//    GET  /api/v1/targets/by-incident-type/{type}    ?include_encodings,page,limit
//    GET  /api/v1/targets/{target_id}
//    GET  /api/v1/targets/{target_id}/encoding
//          → serves .npy file: np.load(io.BytesIO(resp.content)) → (128,) float32
//
//  TARGET FLAGS — write
//    POST /api/v1/targets/{target_id}/flag
//         { flag_type: "poi"|"watchlist"|"wanted"|"pin"|"takedown"|"flagged",
//           reason?, user_id?, display_name?, notes? }
//
//  PERSONS OF INTEREST
//    GET  /api/v1/poi                     ?status,page,limit
//    POST /api/v1/poi                     { display_name, category, notes?,
//                                           linked_evidence_ids?, pinned_by_user_id }
//    POST /api/v1/poi/{id}/status         { status }
//
//  FACE MATCH FEEDBACK
//    POST /api/v1/feedback/face-match
//         { source_target_id, matched_target_id,
//           evidence_id_source, evidence_id_matched,
//           confidence_pct, augmentation_name?, notes? }
//      → links cases + notifies uploader + logs audit
//
//  EVENTS (SSE)
//    GET  /api/v1/events
//
// ═══════════════════════════════════════════════════════════════════════════

use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::stream::unfold;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use chrono::Utc;
use uuid::Uuid;
use actix_web::rt::time::interval;

use crate::database::Database;
use crate::evidence_service::EvidenceService;
use crate::models::EvidenceSearchFilters;

// ═══════════════════════════════════════════════════════════════════════════
// API-KEY GUARD
// ═══════════════════════════════════════════════════════════════════════════

fn validate_api_key(req: &HttpRequest) -> bool {
    let expected = match std::env::var("PYTHON_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            eprintln!("⚠️  PYTHON_API_KEY is not set — all /api/v1 requests rejected");
            return false;
        }
    };
    req.headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == expected)
        .unwrap_or(false)
}

macro_rules! require_api_key {
    ($req:expr) => {
        if !validate_api_key($req) {
            return HttpResponse::Unauthorized().json(json!({
                "success": false,
                "error":   "Invalid or missing X-API-Key header"
            }));
        }
    };
}

// ═══════════════════════════════════════════════════════════════════════════
// HEALTH
// ═══════════════════════════════════════════════════════════════════════════

pub async fn api_v1_health() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status":    "ok",
        "timestamp": Utc::now().to_rfc3339(),
        "version":   "v1"
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// AUTH
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct LoginBody {
    pub email:    String,
    pub password: String,
}

/// POST /api/v1/auth/login
///
/// Verifies email + bcrypt password against the users table.
/// On success returns the full user profile AND the PYTHON_API_KEY value
/// so the Python app can store it and forward it on every subsequent call.
pub async fn api_v1_login(
    database: web::Data<Database>,
    body:     web::Json<LoginBody>,
) -> HttpResponse {
    let user = match database.get_user_by_email(&body.email).await {
        Ok(Some(u)) => u,
        Ok(None)    => return HttpResponse::Unauthorized().json(json!({
            "success": false, "error": "Invalid email or password"
        })),
        Err(e) => return HttpResponse::InternalServerError().json(json!({
            "success": false, "error": format!("DB error: {}", e)
        })),
    };

    let hash = match &user.password_hash {
        Some(h) => h.clone(),
        None    => return HttpResponse::Unauthorized().json(json!({
            "success": false, "error": "Account uses wallet auth only — no password set"
        })),
    };

    if !bcrypt::verify(&body.password, &hash).unwrap_or(false) {
        return HttpResponse::Unauthorized().json(json!({
            "success": false, "error": "Invalid email or password"
        }));
    }

    if !user.is_verified {
        return HttpResponse::Forbidden().json(json!({
            "success": false, "error": "Email address not yet verified"
        }));
    }

    let _ = database.update_user_login(&user.id, None).await;

    let api_key = std::env::var("PYTHON_API_KEY").unwrap_or_default();

    HttpResponse::Ok().json(json!({
        "success": true,
        "api_key": api_key,
        "user": {
            "id":                  user.id,
            "email":               user.email,
            "is_verified":         user.is_verified,
            "account_type":        user.account_type,
            "business_name":       user.business_name,
            "phone_number":        user.phone_number,
            "county":              user.county,
            "id_number":           user.id_number,
            "wallet_address":      user.wallet_address,
            "wallet_type":         user.wallet_type,
            "wallet_chain":        user.wallet_chain,
            "public_key":          user.public_key,
            "geo_latitude":        user.geo_latitude,
            "geo_longitude":       user.geo_longitude,
            "is_profile_complete": user.is_profile_complete,
            "created_at":          user.created_at,
            "updated_at":          user.updated_at,
        }
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// EVIDENCE — READ
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct EvidenceListQuery {
    pub page:            Option<i64>,
    pub limit:           Option<i64>,
    pub status:          Option<String>,
    pub emergency_level: Option<String>,
    pub incident_type:   Option<String>,
    pub country:         Option<String>,
    pub county:          Option<String>,
    pub constituency:    Option<String>,
    pub ward:            Option<String>,
    pub lat:             Option<f64>,
    pub lng:             Option<f64>,
    pub radius_km:       Option<f64>,
    pub query:           Option<String>,
    pub sort_by:         Option<String>,
    pub date_from:       Option<String>,
    pub date_to:         Option<String>,
}

pub async fn api_v1_list_evidence(
    req:              HttpRequest,
    qs:               web::Query<EvidenceListQuery>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    require_api_key!(&req);

    let page  = qs.page.unwrap_or(1).max(1);
    let limit = qs.limit.unwrap_or(50).clamp(1, 200);

    let county_filter = qs.county.clone().or_else(|| qs.constituency.clone());

    let filters = EvidenceSearchFilters {
        query:              qs.query.clone(),
        status:             qs.status.clone(),
        emergency_level:    qs.emergency_level.clone(),
        incident_type:      qs.incident_type.clone(),
        county:             county_filter,
        sort_by:            Some(qs.sort_by.clone().unwrap_or_else(|| "newest".to_string())),
        page,
        limit,
        reported_to_police: None,
        needs_attention:    None,
        signed_only:        None,
        uploader_id:        None,
        date_from:          qs.date_from.clone(),
        date_to:            qs.date_to.clone(),
        start_date:         None,
        end_date:           None,
    };

    match evidence_service.search_evidence_with_filters(&filters, "").await {
        Ok(r) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": {
                "items":       r.summaries,
                "total":       r.total,
                "page":        page,
                "limit":       limit,
                "total_pages": ((r.total as f64) / (limit as f64)).ceil() as i64,
            }
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    }
}

pub async fn api_v1_get_evidence(
    req:              HttpRequest,
    path:             web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    require_api_key!(&req);
    let id = path.into_inner();
    match evidence_service.get_evidence_detail(&id).await {
        Ok(Some(d)) => HttpResponse::Ok().json(json!({ "success": true, "data": d })),
        Ok(None)    => HttpResponse::NotFound().json(json!({ "success": false, "error": "Not found" })),
        Err(e)      => HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    }
}

// ─── Location compound query ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LocationQuery {
    pub country:           Option<String>,
    pub county:            Option<String>,
    pub constituency:      Option<String>,
    pub ward:              Option<String>,
    pub lat:               Option<f64>,
    pub lng:               Option<f64>,
    pub radius_km:         Option<f64>,
    pub incident_type:     Option<String>,
    pub status:            Option<String>,
    pub emergency_level:   Option<String>,
    /// 1 = evidence JSON only  |  2 = evidence + targets + encodings
    pub mode:              Option<u8>,
    pub include_encodings: Option<bool>,
    pub page:              Option<i64>,
    pub limit:             Option<i64>,
}

/// GET /api/v1/evidence/by-location
pub async fn api_v1_evidence_by_location(
    req:              HttpRequest,
    qs:               web::Query<LocationQuery>,
    evidence_service: web::Data<EvidenceService>,
    database:         web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);

    let page  = qs.page.unwrap_or(1).max(1);
    let limit = qs.limit.unwrap_or(50).clamp(1, 200);
    let mode  = qs.mode.unwrap_or(1);
    let include_encodings = qs.include_encodings.unwrap_or(mode == 2);

    let county_filter = qs.county.clone().or_else(|| qs.constituency.clone());

    let filters = EvidenceSearchFilters {
        query:              None,
        status:             qs.status.clone(),
        emergency_level:    qs.emergency_level.clone(),
        incident_type:      qs.incident_type.clone(),
        county:             county_filter,
        sort_by:            Some("newest".to_string()),
        page,
        limit,
        reported_to_police: None,
        needs_attention:    None,
        signed_only:        None,
        uploader_id:        None,
        date_from:          None,
        date_to:            None,
        start_date:         None,
        end_date:           None,
    };

    let result = match evidence_service.search_evidence_with_filters(&filters, "").await {
        Ok(r)  => r,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    // Optional lat/lng post-filter
    let summaries = if let (Some(lat), Some(lng), Some(r_km)) = (qs.lat, qs.lng, qs.radius_km) {
        result.summaries.into_iter().filter(|s| {
            let v = serde_json::to_value(s).unwrap_or_default();
            let ilat = v.get("latitude").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let ilng = v.get("longitude").and_then(|x| x.as_f64()).unwrap_or(0.0);
            haversine_km(lat, lng, ilat, ilng) <= r_km
        }).collect::<Vec<_>>()
    } else {
        result.summaries
    };

    let total_pages = ((result.total as f64) / (limit as f64)).ceil() as i64;

    let location_meta = json!({
        "country":      qs.country,
        "county":       qs.county,
        "constituency": qs.constituency,
        "ward":         qs.ward,
        "lat":          qs.lat,
        "lng":          qs.lng,
        "radius_km":    qs.radius_km,
    });

    if mode == 1 {
        return HttpResponse::Ok().json(json!({
            "success": true,
            "mode":    1,
            "data": {
                "items":           summaries,
                "total":           result.total,
                "page":            page,
                "limit":           limit,
                "total_pages":     total_pages,
                "location_filter": location_meta,
            }
        }));
    }

    // mode == 2 — enrich each evidence record with its targets + encodings
    let mut enriched = Vec::new();
    for summary in &summaries {
        let targets = fetch_targets_for_evidence(&database, &summary.id, include_encodings).await;
        enriched.push(json!({
            "evidence": summary,
            "targets":  targets,
        }));
    }

    HttpResponse::Ok().json(json!({
        "success": true,
        "mode":    2,
        "data": {
            "items":           enriched,
            "total":           result.total,
            "page":            page,
            "limit":           limit,
            "total_pages":     total_pages,
            "location_filter": location_meta,
        }
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// EVIDENCE — WRITE
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize, Serialize)]
pub struct EvidenceUpdateBody {
    pub title:               Option<String>,
    pub description:         Option<String>,
    pub emergency_level:     Option<String>,
    pub county:              Option<String>,
    pub constituency:        Option<String>,
    pub ward:                Option<String>,
    pub landmark:            Option<String>,
    pub suspect_description: Option<String>,
    pub injuries:            Option<String>,
    pub property_damage:     Option<String>,
    pub needs_attention:     Option<bool>,
    pub police_case_id:      Option<String>,
    pub police_station:      Option<String>,
}

pub async fn api_v1_update_evidence(
    req:      HttpRequest,
    path:     web::Path<String>,
    body:     web::Json<EvidenceUpdateBody>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);
    let id = path.into_inner();

    let mut sets: Vec<String> = Vec::new();
    let mut vals: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    macro_rules! push_str {
        ($col:expr, $val:expr) => {
            if let Some(v) = $val {
                sets.push(format!("{} = ?{}", $col, vals.len() + 1));
                vals.push(Box::new(v.clone()));
            }
        };
    }

    push_str!("title",               &body.title);
    push_str!("description",         &body.description);
    push_str!("emergency_level",     &body.emergency_level);
    push_str!("county",              &body.county);
    push_str!("constituency",        &body.constituency);
    push_str!("ward",                &body.ward);
    push_str!("landmark",            &body.landmark);
    push_str!("suspect_description", &body.suspect_description);
    push_str!("injuries",            &body.injuries);
    push_str!("property_damage",     &body.property_damage);
    push_str!("police_case_id",      &body.police_case_id);
    push_str!("police_station",      &body.police_station);

    if let Some(na) = body.needs_attention {
        sets.push(format!("needs_attention = ?{}", vals.len() + 1));
        vals.push(Box::new(if na { 1i64 } else { 0i64 }));
    }

    if sets.is_empty() {
        return HttpResponse::BadRequest().json(json!({ "success": false, "error": "No fields provided" }));
    }

    let now_ts = Utc::now().timestamp();
    sets.push(format!("updated_at = ?{}", vals.len() + 1));
    vals.push(Box::new(now_ts));
    let where_idx = vals.len() + 1;
    vals.push(Box::new(id.clone()));

    let sql = format!("UPDATE evidence SET {} WHERE id = ?{}", sets.join(", "), where_idx);
    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };
    let refs: Vec<&dyn rusqlite::ToSql> = vals.iter().map(|v| v.as_ref()).collect();

    match conn.execute(&sql, refs.as_slice()) {
        Ok(0) => HttpResponse::NotFound().json(json!({ "success": false, "error": "Record not found" })),
        Ok(_) => {
            write_audit(&conn, "python_api", "evidence_updated", "evidence", &id,
                        &serde_json::to_string(&*body).unwrap_or_default());
            HttpResponse::Ok().json(json!({ "success": true, "message": "Updated", "id": id }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct StatusUpdateBody {
    pub status: String,
    pub reason: Option<String>,
}

pub async fn api_v1_update_status(
    req:      HttpRequest,
    path:     web::Path<String>,
    body:     web::Json<StatusUpdateBody>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);
    let id = path.into_inner();

    let valid = ["Draft","Submitted","Reported","UnderReview","Archived","Rejected"];
    if !valid.contains(&body.status.as_str()) {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error":   format!("Invalid status. Must be one of: {}", valid.join(", "))
        }));
    }

    let now_ts = Utc::now().timestamp();
    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    match conn.execute(
        "UPDATE evidence SET status=?1, updated_at=?2 WHERE id=?3",
        rusqlite::params![body.status, now_ts, id],
    ) {
        Ok(0) => HttpResponse::NotFound().json(json!({ "success": false, "error": "Not found" })),
        Ok(_) => {
            write_audit(&conn, "python_api", "status_changed", "evidence", &id,
                        &format!(r#"{{"new_status":"{}","reason":{:?}}}"#, body.status, body.reason));
            HttpResponse::Ok().json(json!({ "success": true, "id": id, "new_status": body.status }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TARGETS — READ
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct TargetQuery {
    pub include_encodings: Option<bool>,
    pub page:              Option<i64>,
    pub limit:             Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct TargetsByLocationQuery {
    pub country:           Option<String>,
    pub county:            Option<String>,
    pub constituency:      Option<String>,
    pub ward:              Option<String>,
    pub lat:               Option<f64>,
    pub lng:               Option<f64>,
    pub radius_km:         Option<f64>,
    pub incident_type:     Option<String>,
    pub category:          Option<String>,
    pub include_encodings: Option<bool>,
    pub page:              Option<i64>,
    pub limit:             Option<i64>,
}

/// GET /api/v1/targets/by-evidence/{evidence_id}
pub async fn api_v1_targets_by_evidence(
    req:      HttpRequest,
    path:     web::Path<String>,
    qs:       web::Query<TargetQuery>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);
    let evidence_id       = path.into_inner();
    let include_encodings = qs.include_encodings.unwrap_or(true);
    let targets = fetch_targets_for_evidence(&database, &evidence_id, include_encodings).await;
    HttpResponse::Ok().json(json!({
        "success":     true,
        "evidence_id": evidence_id,
        "count":       targets.len(),
        "data":        targets,
    }))
}

/// GET /api/v1/targets/by-location
///
/// Every target whose parent evidence matches the supplied location fields.
/// Returns raw_image_url (Storj) + encoding_b64 (base64 of 512-byte f32 blob)
/// for each target.
pub async fn api_v1_targets_by_location(
    req:      HttpRequest,
    qs:       web::Query<TargetsByLocationQuery>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);

    let page   = qs.page.unwrap_or(1).max(1) as i64;
    let limit  = qs.limit.unwrap_or(100).clamp(1, 500) as i64;
    let offset = (page - 1) * limit;
    let inc    = qs.include_encodings.unwrap_or(true);

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    let mut wheres: Vec<String>                 = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    macro_rules! push_loc {
        ($col:expr, $val:expr) => {
            if let Some(ref v) = $val {
                wheres.push(format!("LOWER({}) = LOWER(?{})", $col, params.len() + 1));
                params.push(Box::new(v.clone()));
            }
        };
    }

    push_loc!("e.county",        qs.county);
    push_loc!("e.constituency",  qs.constituency);
    push_loc!("e.ward",          qs.ward);
    push_loc!("e.incident_type", qs.incident_type);
    push_loc!("t.category",      qs.category);

    let where_sql = if wheres.is_empty() { String::new() } else { format!("WHERE {}", wheres.join(" AND ")) };

    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let sql = format!(
        r#"
        SELECT t.id, t.evidence_id, t.target_number, t.filename, t.mime_type,
               t.file_size, t.description, t.category, t.confidence_score,
               t.storj_url, t.hash, t.phash, t.auto_generated, t.created_at, t.created_by,
               e.county, e.constituency, e.ward, e.latitude, e.longitude,
               e.incident_type, e.emergency_level, e.evidence_number,
               e.uploader_id, e.uploader_email, e.title, e.incident_time, e.status
        FROM targets t
        JOIN evidence e ON t.evidence_id = e.id
        {}
        ORDER BY t.created_at DESC
        LIMIT ?{} OFFSET ?{}
        "#,
        where_sql,
        params.len() - 1,
        params.len(),
    );

    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = match conn.prepare(&sql) {
        Ok(mut stmt) => stmt.query_map(refs.as_slice(), target_row_to_json)
            .map(|m| m.filter_map(|r| r.ok()).collect::<Vec<_>>())
            .unwrap_or_default(),
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    // Optional lat/lng post-filter
    let rows = if let (Some(lat), Some(lng), Some(r_km)) = (qs.lat, qs.lng, qs.radius_km) {
        rows.into_iter().filter(|t| {
            let tlat = t["location"]["latitude"].as_f64().unwrap_or(0.0);
            let tlng = t["location"]["longitude"].as_f64().unwrap_or(0.0);
            haversine_km(lat, lng, tlat, tlng) <= r_km
        }).collect::<Vec<_>>()
    } else {
        rows
    };

    let targets_json = if inc {
        let mut out = Vec::new();
        for mut t in rows {
            let tid = t["id"].as_str().unwrap_or("").to_string();
            let eid = t["evidence_id"].as_str().unwrap_or("").to_string();
            t["encodings"] = json!(fetch_encodings_for_target(&database, &tid, &eid).await);
            out.push(t);
        }
        out
    } else {
        rows
    };

    HttpResponse::Ok().json(json!({
        "success": true,
        "count":   targets_json.len(),
        "page":    page,
        "limit":   limit,
        "location_filter": {
            "country": qs.country, "county": qs.county,
            "constituency": qs.constituency, "ward": qs.ward,
            "lat": qs.lat, "lng": qs.lng, "radius_km": qs.radius_km,
        },
        "data": targets_json,
    }))
}

/// GET /api/v1/targets/by-incident-type/{type}
pub async fn api_v1_targets_by_incident_type(
    req:      HttpRequest,
    path:     web::Path<String>,
    qs:       web::Query<TargetQuery>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);

    let incident_type = path.into_inner();
    let inc    = qs.include_encodings.unwrap_or(true);
    let page   = qs.page.unwrap_or(1).max(1) as i64;
    let limit  = qs.limit.unwrap_or(100).clamp(1, 500) as i64;
    let offset = (page - 1) * limit;

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    let rows = match conn.prepare(
        r#"
        SELECT t.id, t.evidence_id, t.target_number, t.filename, t.mime_type,
               t.file_size, t.description, t.category, t.confidence_score,
               t.storj_url, t.hash, t.phash, t.auto_generated, t.created_at, t.created_by,
               e.county, e.constituency, e.ward, e.latitude, e.longitude,
               e.incident_type, e.emergency_level, e.evidence_number,
               e.uploader_id, e.uploader_email, e.title, e.incident_time, e.status
        FROM targets t
        JOIN evidence e ON t.evidence_id = e.id
        WHERE LOWER(e.incident_type) = LOWER(?1)
        ORDER BY t.created_at DESC
        LIMIT ?2 OFFSET ?3
        "#,
    ) {
        Ok(mut s) => s.query_map(rusqlite::params![incident_type, limit, offset], target_row_to_json)
            .map(|m| m.filter_map(|r| r.ok()).collect::<Vec<_>>())
            .unwrap_or_default(),
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    let targets_json = if inc {
        let mut out = Vec::new();
        for mut t in rows {
            let tid = t["id"].as_str().unwrap_or("").to_string();
            let eid = t["evidence_id"].as_str().unwrap_or("").to_string();
            t["encodings"] = json!(fetch_encodings_for_target(&database, &tid, &eid).await);
            out.push(t);
        }
        out
    } else { rows };

    HttpResponse::Ok().json(json!({
        "success": true,
        "incident_type": incident_type,
        "count": targets_json.len(),
        "page": page, "limit": limit,
        "data": targets_json,
    }))
}

/// GET /api/v1/targets/{target_id}
pub async fn api_v1_get_target(
    req:      HttpRequest,
    path:     web::Path<String>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);
    let target_id = path.into_inner();

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    let row = conn.query_row(
        r#"
        SELECT t.id, t.evidence_id, t.target_number, t.filename, t.mime_type,
               t.file_size, t.description, t.category, t.confidence_score,
               t.storj_url, t.hash, t.phash, t.auto_generated, t.created_at, t.created_by,
               e.county, e.constituency, e.ward, e.latitude, e.longitude,
               e.incident_type, e.emergency_level, e.evidence_number,
               e.uploader_id, e.uploader_email, e.title, e.incident_time, e.status
        FROM targets t JOIN evidence e ON t.evidence_id = e.id WHERE t.id = ?1
        "#,
        rusqlite::params![target_id],
        target_row_to_json,
    );

    match row {
        Ok(mut data) => {
            let tid = data["id"].as_str().unwrap_or("").to_string();
            let eid = data["evidence_id"].as_str().unwrap_or("").to_string();
            data["encodings"] = json!(fetch_encodings_for_target(&database, &tid, &eid).await);
            HttpResponse::Ok().json(json!({ "success": true, "data": data }))
        }
        Err(rusqlite::Error::QueryReturnedNoRows) =>
            HttpResponse::NotFound().json(json!({ "success": false, "error": "Target not found" })),
        Err(e) =>
            HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    }
}

/// GET /api/v1/targets/{target_id}/encoding
///
/// Serves a numpy .npy file for the target's face encoding.
///
/// Priority:
///   1. `encodings/{evidence_id}/{target_id}_0.pkl` on disk   → stream as-is
///   2. face_encodings DB blob                                  → synthesise .npy
///
/// Python usage:
///   import io, numpy as np, requests
///   arr = np.load(io.BytesIO(requests.get(url, headers=...).content))
///   # arr.shape == (128,), dtype float32
pub async fn api_v1_target_encoding(
    req:      HttpRequest,
    path:     web::Path<String>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);
    let target_id = path.into_inner();

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    let evidence_id: Option<String> = conn.query_row(
        "SELECT evidence_id FROM targets WHERE id = ?1",
        rusqlite::params![target_id],
        |row| row.get(0),
    ).optional().unwrap_or(None);

    let evidence_id = match evidence_id {
        Some(id) => id,
        None     => return HttpResponse::NotFound().json(json!({ "success": false, "error": "Target not found" })),
    };

    // Path 1: local disk pickle (written by face sidecar)
    let pkl_path = format!("encodings/{}/{}_0.pkl", evidence_id, target_id);
    if let Ok(bytes) = std::fs::read(&pkl_path) {
        let fname = format!("{}_0.pkl", &target_id[..target_id.len().min(16)]);
        return HttpResponse::Ok()
            .content_type("application/octet-stream")
            .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", fname)))
            .insert_header(("X-Encoding-Source", "disk"))
            .body(bytes);
    }

    // Path 2: synthesise numpy .npy from DB blob
    let blob: Option<Vec<u8>> = conn.query_row(
        "SELECT descriptor FROM face_encodings WHERE target_id = ?1 ORDER BY face_index ASC LIMIT 1",
        rusqlite::params![target_id],
        |row| row.get(0),
    ).optional().unwrap_or(None);

    match blob {
        None => HttpResponse::NotFound().json(json!({ "success": false, "error": "No encoding for this target" })),
        Some(bytes) => {
            let npy = make_npy_buffer(&bytes);
            let fname = format!("{}_0.npy", &target_id[..target_id.len().min(16)]);
            HttpResponse::Ok()
                .content_type("application/octet-stream")
                .insert_header(("Content-Disposition", format!("attachment; filename=\"{}\"", fname)))
                .insert_header(("X-Encoding-Source", "database"))
                .body(npy)
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TARGET FLAGS — WRITE
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct FlagTargetBody {
    /// poi | watchlist | wanted | pin | takedown | flagged
    pub flag_type:    String,
    pub reason:       Option<String>,
    /// FLUG user_id this flag is attributed to
    pub user_id:      Option<String>,
    pub display_name: Option<String>,
    pub notes:        Option<String>,
}

/// POST /api/v1/targets/{target_id}/flag
///
/// Writes to target_flags (same table as target_routes.rs).
/// For poi/wanted: also upserts a persons_of_interest row.
/// Always fires a notification to the evidence uploader.
pub async fn api_v1_flag_target(
    req:      HttpRequest,
    path:     web::Path<String>,
    body:     web::Json<FlagTargetBody>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);
    let target_id = path.into_inner();

    let valid = ["poi","watchlist","wanted","pin","takedown","flagged"];
    if !valid.contains(&body.flag_type.as_str()) {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error":   format!("flag_type must be one of: {}", valid.join(", "))
        }));
    }

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    // Fetch target + uploader info
    let info: Option<(String, String, String, String)> = conn.query_row(
        r#"SELECT t.evidence_id, t.hash, e.uploader_id, e.uploader_email
           FROM targets t JOIN evidence e ON t.evidence_id = e.id WHERE t.id = ?1"#,
        rusqlite::params![target_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    ).optional().unwrap_or(None);

    let (evidence_id, target_hash, uploader_id, uploader_email) = match info {
        Some(t) => t,
        None    => return HttpResponse::NotFound().json(json!({ "success": false, "error": "Target not found" })),
    };

    let now_ts        = Utc::now().timestamp();
    let flag_id       = format!("flag_{}", Uuid::new_v4());
    let attributed_to = body.user_id.as_deref().unwrap_or("python_api");

    // Upsert into target_flags
    let _ = conn.execute(
        r#"INSERT INTO target_flags (id, target_id, evidence_id, flag_type, flagged_by, reason, created_at)
           VALUES (?1,?2,?3,?4,?5,?6,?7)
           ON CONFLICT(target_id, flag_type) DO UPDATE SET
               reason=excluded.reason, flagged_by=excluded.flagged_by, created_at=excluded.created_at"#,
        rusqlite::params![flag_id, target_id, evidence_id, body.flag_type, attributed_to, body.reason, now_ts],
    );

    // For poi/wanted: upsert persons_of_interest
    let poi_id = if body.flag_type == "poi" || body.flag_type == "wanted" {
        let status  = if body.flag_type == "wanted" { "active" } else { "watching" };
        let name    = body.display_name.clone()
            .unwrap_or_else(|| format!("Target {}", &target_id[..8.min(target_id.len())]));
        let poi_uid = format!("poi_{}", Uuid::new_v4());
        let poi_num = format!("POI-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());
        let ev_json = serde_json::to_string(&vec![&evidence_id]).unwrap_or_default();

        let _ = conn.execute(
            r#"INSERT INTO persons_of_interest
                   (id,poi_number,display_name,category,status,linked_cases,
                    linked_evidence,notes,pinned_by,created_at,last_seen_at)
               VALUES (?1,?2,?3,'person',?4,1,?5,?6,?7,?8,?8)
               ON CONFLICT DO NOTHING"#,
            rusqlite::params![poi_uid, poi_num, name, status, ev_json, body.notes, attributed_to, now_ts],
        );
        Some(poi_uid)
    } else { None };

    // Audit
    write_audit(&conn, "python_api", &format!("target_{}", body.flag_type),
                "target", &target_id,
                &format!(r#"{{"flag_type":"{}","reason":{:?},"by":"{}"}}"#,
                          body.flag_type, body.reason, attributed_to));

    // Notify uploader
    let title = match body.flag_type.as_str() {
        "poi"       => "Target Flagged as Person of Interest",
        "wanted"    => "Target Marked as Wanted",
        "watchlist" => "Target Added to Watchlist",
        "takedown"  => "Target Flagged for Takedown",
        _           => "Target Flagged",
    };
    let msg = format!(
        "A target from your case was marked as '{}' by the intelligence system.",
        body.flag_type
    );
    let _ = database.create_notification(
        &uploader_id, "python_flag", title, &msg,
        Some(&evidence_id), None, Some(&target_hash),
    ).await;

    HttpResponse::Ok().json(json!({
        "success":     true,
        "flag_id":     flag_id,
        "target_id":   target_id,
        "evidence_id": evidence_id,
        "flag_type":   body.flag_type,
        "poi_id":      poi_id,
        "uploader_notified": { "user_id": uploader_id, "email": uploader_email },
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// PERSONS OF INTEREST
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct PoiListQuery {
    pub status: Option<String>,
    pub page:   Option<i64>,
    pub limit:  Option<i64>,
}

/// GET /api/v1/poi
pub async fn api_v1_list_poi(
    req:      HttpRequest,
    qs:       web::Query<PoiListQuery>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);

    let page   = qs.page.unwrap_or(1).max(1) as i64;
    let limit  = qs.limit.unwrap_or(50).clamp(1, 200) as i64;
    let offset = (page - 1) * limit;

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    let mut wheres: Vec<String>                 = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(ref s) = qs.status {
        wheres.push(format!("status = ?{}", params.len() + 1));
        params.push(Box::new(s.clone()));
    }

    let where_sql = if wheres.is_empty() { String::new() } else { format!("WHERE {}", wheres.join(" AND ")) };
    params.push(Box::new(limit));
    params.push(Box::new(offset));

    let sql = format!(
        "SELECT id,poi_number,display_name,category,status,linked_cases,
                linked_evidence,notes,pinned_by,created_at,last_seen_at,resolved_at
         FROM persons_of_interest {} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        where_sql, params.len() - 1, params.len()
    );

    let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let rows = match conn.prepare(&sql) {
        Ok(mut stmt) => stmt.query_map(refs.as_slice(), |row| {
            Ok(json!({
                "id":              row.get::<_,String>(0)?,
                "poi_number":      row.get::<_,String>(1)?,
                "display_name":    row.get::<_,String>(2)?,
                "category":        row.get::<_,String>(3)?,
                "status":          row.get::<_,String>(4)?,
                "linked_cases":    row.get::<_,i64>(5)?,
                "linked_evidence": row.get::<_,String>(6)?,
                "notes":           row.get::<_,Option<String>>(7)?,
                "pinned_by":       row.get::<_,String>(8)?,
                "created_at":      row.get::<_,i64>(9)?,
                "last_seen_at":    row.get::<_,Option<i64>>(10)?,
                "resolved_at":     row.get::<_,Option<i64>>(11)?,
            }))
        }).map(|m| m.filter_map(|r| r.ok()).collect::<Vec<_>>()).unwrap_or_default(),
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    HttpResponse::Ok().json(json!({ "success": true, "count": rows.len(), "page": page, "limit": limit, "data": rows }))
}

#[derive(Debug, Deserialize)]
pub struct CreatePoiBody {
    pub display_name:        String,
    pub category:            String,
    pub notes:               Option<String>,
    pub linked_evidence_ids: Option<Vec<String>>,
    pub pinned_by_user_id:   String,
}

/// POST /api/v1/poi
pub async fn api_v1_create_poi(
    req:      HttpRequest,
    body:     web::Json<CreatePoiBody>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);

    let valid_cats = ["person","vehicle","unknown"];
    if !valid_cats.contains(&body.category.as_str()) {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error":   format!("category must be one of: {}", valid_cats.join(", "))
        }));
    }

    let poi_id  = format!("poi_{}", Uuid::new_v4());
    let poi_num = format!("POI-{}", &Uuid::new_v4().to_string()[..8].to_uppercase());
    let ev_ids  = body.linked_evidence_ids.clone().unwrap_or_default();
    let ev_json = serde_json::to_string(&ev_ids).unwrap_or_default();
    let now_ts  = Utc::now().timestamp();

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    match conn.execute(
        r#"INSERT INTO persons_of_interest
               (id,poi_number,display_name,category,status,linked_cases,
                linked_evidence,notes,pinned_by,created_at)
           VALUES (?1,?2,?3,?4,'watching',?5,?6,?7,?8,?9)"#,
        rusqlite::params![poi_id, poi_num, body.display_name, body.category,
                          ev_ids.len() as i64, ev_json, body.notes, body.pinned_by_user_id, now_ts],
    ) {
        Ok(_) => {
            write_audit(&conn, "python_api", "poi_created", "poi", &poi_id,
                        &format!(r#"{{"name":"{}"}}"#, body.display_name));
            HttpResponse::Ok().json(json!({ "success": true, "poi_id": poi_id, "poi_number": poi_num }))
        }
        Err(e) => HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    }
}

#[derive(Debug, Deserialize)]
pub struct PoiStatusBody { pub status: String }

/// POST /api/v1/poi/{id}/status
pub async fn api_v1_update_poi_status(
    req:      HttpRequest,
    path:     web::Path<String>,
    body:     web::Json<PoiStatusBody>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);
    let poi_id = path.into_inner();

    let valid = ["watching","active","resolved","archived"];
    if !valid.contains(&body.status.as_str()) {
        return HttpResponse::BadRequest().json(json!({
            "success": false,
            "error":   format!("status must be one of: {}", valid.join(", "))
        }));
    }

    let now_ts   = Utc::now().timestamp();
    let resolved: Option<i64> = if body.status == "resolved" { Some(now_ts) } else { None };

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    match conn.execute(
        "UPDATE persons_of_interest SET status=?1,resolved_at=?2,last_seen_at=?3 WHERE id=?4",
        rusqlite::params![body.status, resolved, now_ts, poi_id],
    ) {
        Ok(0) => HttpResponse::NotFound().json(json!({ "success": false, "error": "POI not found" })),
        Ok(_) => HttpResponse::Ok().json(json!({ "success": true, "poi_id": poi_id, "new_status": body.status })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FACE MATCH FEEDBACK
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct FaceMatchFeedback {
    pub source_target_id:    String,
    pub matched_target_id:   String,
    pub evidence_id_source:  String,
    pub evidence_id_matched: String,
    /// 0–100
    pub confidence_pct:      f64,
    /// Name of the Python pipeline, e.g. "DeepFace v0.3 — Nairobi scan"
    pub augmentation_name:   Option<String>,
    pub notes:               Option<String>,
}

/// POST /api/v1/feedback/face-match
///
/// Called by the Python pipeline when it finds a face match.
/// Actions:
///   1. Link the two evidence cases (linked_cases table)
///   2. Set needs_attention=1 on the matched evidence record
///   3. Write a detailed audit row crediting the augmentation pipeline
///   4. Fire a notification to the evidence uploader:
///      "Case match found by <aug_name> at <confidence>%"
pub async fn api_v1_face_match_feedback(
    req:      HttpRequest,
    body:     web::Json<FaceMatchFeedback>,
    database: web::Data<Database>,
) -> HttpResponse {
    require_api_key!(&req);

    if !(0.0..=100.0).contains(&body.confidence_pct) {
        return HttpResponse::BadRequest().json(json!({ "success": false, "error": "confidence_pct must be 0–100" }));
    }

    let conn = match database.pool.get() {
        Ok(c)  => c,
        Err(e) => return HttpResponse::InternalServerError().json(json!({ "success": false, "error": e.to_string() })),
    };

    // Validate matched target and get uploader info
    let matched: Option<(String, String, String)> = conn.query_row(
        "SELECT t.hash, e.uploader_id, e.uploader_email FROM targets t JOIN evidence e ON t.evidence_id = e.id WHERE t.id = ?1",
        rusqlite::params![body.matched_target_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).optional().unwrap_or(None);

    let (target_hash, uploader_id, uploader_email) = match matched {
        Some(m) => m,
        None    => return HttpResponse::NotFound().json(json!({ "success": false, "error": "matched_target_id not found" })),
    };

    let aug = body.augmentation_name.as_deref().unwrap_or("Python Intelligence Pipeline");
    let now_ts = Utc::now().timestamp();

    // 1. Link cases
    let link_reason = format!(
        "Face match by {} — {:.1}% confidence. Source: {}, Matched: {}",
        aug, body.confidence_pct, body.source_target_id, body.matched_target_id
    );
    let link_id = database.link_evidence_cases(
        &body.evidence_id_source,
        &body.evidence_id_matched,
        "face_match",
        &link_reason,
        &target_hash,
        body.confidence_pct.round() as i32,
        Some("python_api"),
    ).await.unwrap_or_default();

    // 2. Flag matched evidence as needing attention
    let _ = conn.execute(
        "UPDATE evidence SET needs_attention=1, updated_at=?1 WHERE id=?2",
        rusqlite::params![now_ts, body.evidence_id_matched],
    );

    // 3. Detailed audit log
    write_audit(&conn, "python_api", "face_match_result", "target", &body.matched_target_id,
                &json!({
                    "augmentation_name":  aug,
                    "source_target_id":   body.source_target_id,
                    "matched_target_id":  body.matched_target_id,
                    "ev_source":          body.evidence_id_source,
                    "ev_matched":         body.evidence_id_matched,
                    "confidence_pct":     body.confidence_pct,
                    "notes":              body.notes,
                    "link_id":            link_id,
                }).to_string());

    // 4. Notify uploader
    let notif_title = format!("Case Match Found — {:.0}% Confidence", body.confidence_pct);
    let notif_msg   = format!(
        "{} matched your evidence to another case at {:.1}% confidence.{}",
        aug, body.confidence_pct,
        body.notes.as_deref().map(|n| format!(" Note: {}", n)).unwrap_or_default()
    );
    let notif_id = database.create_notification(
        &uploader_id, "face_match", &notif_title, &notif_msg,
        Some(&body.evidence_id_matched), Some(&body.evidence_id_source), Some(&target_hash),
    ).await.unwrap_or_default();

    HttpResponse::Ok().json(json!({
        "success":           true,
        "link_id":           link_id,
        "notification_id":   notif_id,
        "augmentation":      aug,
        "confidence_pct":    body.confidence_pct,
        "evidence_matched":  body.evidence_id_matched,
        "evidence_source":   body.evidence_id_source,
        "uploader_notified": { "user_id": uploader_id, "email": uploader_email },
    }))
}

// ═══════════════════════════════════════════════════════════════════════════
// SSE EVENT STREAM
// ═══════════════════════════════════════════════════════════════════════════

/// GET /api/v1/events — tails audit_log, heartbeat every 3 s
pub async fn api_v1_events(
    req:      HttpRequest,
    database: web::Data<Database>,
) -> HttpResponse {
    if !validate_api_key(&req) {
        return HttpResponse::Unauthorized().json(json!({ "success": false, "error": "Invalid or missing X-API-Key" }));
    }

    let pool     = database.pool.clone();
    let start_ts = Utc::now().timestamp();

    #[derive(Serialize)]
    struct AuditRow {
        id:            String,
        action_type:   String,
        action_target: String,
        target_id:     Option<String>,
        details:       String,
        created_at:    i64,
    }

    let stream = unfold(start_ts, move |last_seen| {
        let pool = pool.clone();
        async move {
            let mut ticker = interval(Duration::from_secs(3));
            ticker.tick().await;
            ticker.tick().await;

            let conn = match pool.get() {
                Ok(c)  => c,
                Err(_) => return Some((": error\n\n".to_string(), last_seen)),
            };

            let rows: Vec<AuditRow> = conn.prepare(
                "SELECT id,action_type,action_target,target_id,details,created_at \
                 FROM audit_log WHERE created_at > ?1 ORDER BY created_at ASC LIMIT 50"
            ).ok().and_then(|mut s|
                s.query_map(rusqlite::params![last_seen], |row| {
                    Ok(AuditRow {
                        id:            row.get(0)?,
                        action_type:   row.get(1)?,
                        action_target: row.get(2)?,
                        target_id:     row.get(3)?,
                        details:       row.get(4)?,
                        created_at:    row.get(5)?,
                    })
                }).ok().map(|m| m.filter_map(|r| r.ok()).collect())
            ).unwrap_or_default();

            if rows.is_empty() {
                return Some((": ping\n\n".to_string(), last_seen));
            }

            let new_last = rows.last().map(|r| r.created_at).unwrap_or(last_seen);
            let payload  = rows.iter()
                .map(|r| format!("event: audit_log\ndata: {}\n\n", serde_json::to_string(r).unwrap_or_default()))
                .collect::<String>();

            Some((payload, new_last))
        }
    });

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control",     "no-cache"))
        .insert_header(("X-Accel-Buffering", "no"))
        .insert_header(("Access-Control-Allow-Origin", "*"))
        .streaming(stream)
}

// ═══════════════════════════════════════════════════════════════════════════
// INTERNAL HELPERS
// ═══════════════════════════════════════════════════════════════════════════

fn write_audit(conn: &rusqlite::Connection, user_id: &str,
               action_type: &str, action_target: &str,
               target_id: &str, details: &str) {
    let id  = format!("audit_{}", Uuid::new_v4());
    let now = Utc::now().timestamp();
    let _   = conn.execute(
        "INSERT INTO audit_log (id,user_id,action_type,action_target,target_id,ip_address,details,created_at) \
         VALUES (?1,?2,?3,?4,?5,'external',?6,?7)",
        rusqlite::params![id, user_id, action_type, action_target, target_id, details, now],
    );
}

/// Convert a targets+evidence JOIN row to JSON.
/// Column order must exactly match the SELECT used in targets queries.
fn target_row_to_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<serde_json::Value> {
    Ok(json!({
        "id":               row.get::<_,String>(0)?,
        "evidence_id":      row.get::<_,String>(1)?,
        "target_number":    row.get::<_,i64>(2)?,
        "filename":         row.get::<_,String>(3)?,
        "mime_type":        row.get::<_,String>(4)?,
        "file_size":        row.get::<_,i64>(5)?,
        "description":      row.get::<_,Option<String>>(6)?,
        "category":         row.get::<_,String>(7)?,
        "confidence_score": row.get::<_,i32>(8)?,
        "raw_image_url":    row.get::<_,String>(9)?,   // Storj URL — download directly
        "hash":             row.get::<_,String>(10)?,
        "phash":            row.get::<_,Option<String>>(11)?,
        "auto_generated":   row.get::<_,i32>(12)? != 0,
        "created_at":       row.get::<_,i64>(13)?,
        "created_by":       row.get::<_,String>(14)?,
        "location": {
            "county":       row.get::<_,String>(15)?,
            "constituency": row.get::<_,Option<String>>(16)?,
            "ward":         row.get::<_,Option<String>>(17)?,
            "latitude":     row.get::<_,f64>(18)?,
            "longitude":    row.get::<_,f64>(19)?,
        },
        "evidence": {
            "incident_type":   row.get::<_,String>(20)?,
            "emergency_level": row.get::<_,String>(21)?,
            "evidence_number": row.get::<_,String>(22)?,
            "uploader_id":     row.get::<_,String>(23)?,
            "uploader_email":  row.get::<_,String>(24)?,
            "title":           row.get::<_,String>(25)?,
            "incident_time":   row.get::<_,i64>(26)?,
            "status":          row.get::<_,String>(27)?,
        }
    }))
}

/// Fetch all targets for an evidence record, optionally with encodings.
async fn fetch_targets_for_evidence(
    database:  &Database,
    ev_id:     &str,
    with_enc:  bool,
) -> Vec<serde_json::Value> {
    let conn = match database.pool.get() { Ok(c) => c, Err(_) => return vec![] };

    let rows: Vec<serde_json::Value> = conn.prepare(
        r#"SELECT t.id, t.evidence_id, t.target_number, t.filename, t.mime_type,
                  t.file_size, t.description, t.category, t.confidence_score,
                  t.storj_url, t.hash, t.phash, t.auto_generated, t.created_at, t.created_by,
                  e.county, e.constituency, e.ward, e.latitude, e.longitude,
                  e.incident_type, e.emergency_level, e.evidence_number,
                  e.uploader_id, e.uploader_email, e.title, e.incident_time, e.status
           FROM targets t JOIN evidence e ON t.evidence_id = e.id
           WHERE t.evidence_id = ?1 ORDER BY t.target_number ASC"#
    ).ok().and_then(|mut s|
        s.query_map(rusqlite::params![ev_id], target_row_to_json)
         .ok().map(|m| m.filter_map(|r| r.ok()).collect())
    ).unwrap_or_default();

    if !with_enc { return rows; }

    let mut out = Vec::with_capacity(rows.len());
    for mut t in rows {
        let tid = t["id"].as_str().unwrap_or("").to_string();
        let eid = t["evidence_id"].as_str().unwrap_or("").to_string();
        t["encodings"] = json!(fetch_encodings_for_target(database, &tid, &eid).await);
        out.push(t);
    }
    out
}

/// Fetch face encodings for a single target from face_encodings table.
///
/// Each returned object:
///   face_index      — 0 = first/largest face in image
///   detection_score — face-api.js confidence 0.0–1.0
///   encoding_b64    — base64(512 bytes LE f32) — 128-dim vector
///   encoding_path   — local disk path hint for the pickle file
///   descriptor_dims — always 128 (sanity check for Python)
///
/// Python decode:
///   import base64, numpy as np
///   arr = np.frombuffer(base64.b64decode(enc["encoding_b64"]), dtype=np.float32)
///   # arr.shape == (128,)
async fn fetch_encodings_for_target(
    database:  &Database,
    target_id: &str,
    ev_id:     &str,
) -> Vec<serde_json::Value> {
    use base64::Engine as _;

    let conn = match database.pool.get() { Ok(c) => c, Err(_) => return vec![] };

    conn.prepare(
        "SELECT face_index, descriptor, detection_score, phash, auto_generated \
         FROM face_encodings WHERE target_id = ?1 ORDER BY face_index ASC"
    ).ok().and_then(|mut s|
        s.query_map(rusqlite::params![target_id], |row| {
            let face_index:  i32     = row.get(0)?;
            let blob:        Vec<u8> = row.get(1)?;
            let det_score:   f64     = row.get(2)?;
            let phash:       Option<String> = row.get(3)?;
            let auto_gen:    i32     = row.get(4)?;

            let enc_b64 = base64::engine::general_purpose::STANDARD.encode(&blob);
            let path    = format!("encodings/{}/{}_{}.pkl", ev_id, target_id, face_index);

            Ok(json!({
                "face_index":       face_index,
                "detection_score":  det_score,
                "encoding_b64":     enc_b64,
                "encoding_path":    path,
                "phash":            phash,
                "auto_generated":   auto_gen != 0,
                "descriptor_dims":  blob.len() / 4,  // should always be 128
            }))
        }).ok().map(|m| m.filter_map(|r| r.ok()).collect())
    ).unwrap_or_default()
}

/// Haversine distance in km.
fn haversine_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6371.0;
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlon / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().atan2((1.0 - a).sqrt())
}

/// Wrap a raw 512-byte LE float32 blob in a numpy .npy v1.0 header.
/// Python: `arr = np.load(io.BytesIO(resp.content))` → ndarray (128,) float32
fn make_npy_buffer(raw_f32_bytes: &[u8]) -> Vec<u8> {
    let header_str = "{'descr': '<f4', 'fortran_order': False, 'shape': (128,), }";
    let prefix_len  = 10usize;
    let raw_hdr_len = header_str.len() + 1;
    let padded_len  = ((prefix_len + raw_hdr_len + 63) / 64) * 64 - prefix_len;
    let spaces      = padded_len - raw_hdr_len;

    let mut header = header_str.to_string();
    for _ in 0..spaces { header.push(' '); }
    header.push('\n');

    let mut buf = Vec::with_capacity(10 + header.len() + raw_f32_bytes.len());
    buf.extend_from_slice(b"\x93NUMPY");
    buf.push(1); buf.push(0);
    buf.extend_from_slice(&(header.len() as u16).to_le_bytes());
    buf.extend_from_slice(header.as_bytes());
    buf.extend_from_slice(raw_f32_bytes);
    buf
}

// ═══════════════════════════════════════════════════════════════════════════
// ROUTE CONFIG
// ═══════════════════════════════════════════════════════════════════════════

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        // Health (no auth)
        .route("/api/v1/health",                            web::get().to(api_v1_health))
        // Auth (email+password, no X-API-Key)
        .route("/api/v1/auth/login",                        web::post().to(api_v1_login))
        // Evidence — read
        .route("/api/v1/evidence",                          web::get().to(api_v1_list_evidence))
        .route("/api/v1/evidence/by-location",              web::get().to(api_v1_evidence_by_location))
        .route("/api/v1/evidence/{id}",                     web::get().to(api_v1_get_evidence))
        // Evidence — write
        .route("/api/v1/evidence/{id}/update",              web::post().to(api_v1_update_evidence))
        .route("/api/v1/evidence/{id}/status",              web::post().to(api_v1_update_status))
        // Targets — read (specific routes BEFORE /{target_id} to avoid shadowing)
        .route("/api/v1/targets/by-evidence/{evidence_id}", web::get().to(api_v1_targets_by_evidence))
        .route("/api/v1/targets/by-location",               web::get().to(api_v1_targets_by_location))
        .route("/api/v1/targets/by-incident-type/{type}",   web::get().to(api_v1_targets_by_incident_type))
        .route("/api/v1/targets/{target_id}/encoding",      web::get().to(api_v1_target_encoding))
        .route("/api/v1/targets/{target_id}",               web::get().to(api_v1_get_target))
        // Target flags — write
        .route("/api/v1/targets/{target_id}/flag",          web::post().to(api_v1_flag_target))
        // POI
        .route("/api/v1/poi",                               web::get().to(api_v1_list_poi))
        .route("/api/v1/poi",                               web::post().to(api_v1_create_poi))
        .route("/api/v1/poi/{id}/status",                   web::post().to(api_v1_update_poi_status))
        // Face match feedback
        .route("/api/v1/feedback/face-match",               web::post().to(api_v1_face_match_feedback))
        // SSE
        .route("/api/v1/events",                            web::get().to(api_v1_events));
}