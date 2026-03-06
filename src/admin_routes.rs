// src/admin_routes.rs - FIXED VERSION
use actix_web::{web, HttpResponse, Responder}; // Added Responder
use actix_session::Session;
use serde::{Serialize, Deserialize};
use serde_json::json;

use crate::database::Database;

// Define ApiResponse here since it doesn't exist elsewhere
#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct DatabaseStats {
    pub total_users: i64,
    pub total_content: i64,
    pub total_views: i64,
    pub total_likes: i64,
    pub total_wallet_connections: i64,
    pub total_audit_logs: i64,
    pub database_size_bytes: u64,
}

#[derive(Serialize)]
pub struct CleanupStats {
    pub deleted_users: i64,
    pub deleted_old_content: i64,
    pub deleted_audit_logs: i64,
    pub deleted_temp_files: i64,
}

pub async fn get_audit_logs(
    session: Session,
    database: web::Data<Database>,
    query: web::Query<AuditLogQuery>,
) -> HttpResponse {
    // Check admin permissions
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        // In production, check if user is admin
        if !email.ends_with("@admin.com") { // Example admin check
            return HttpResponse::Forbidden().body("Admin access required");
        }
    } else {
        return HttpResponse::Unauthorized().body("Not authenticated");
    }
    
    match database.get_audit_logs(
        query.user_id.as_deref(),
        query.action_type.as_deref(),
        query.limit.unwrap_or(100),
    ).await {
        Ok(logs) => HttpResponse::Ok().json(logs),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

pub async fn get_database_stats(
    _session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    match database.get_database_stats().await {
        Ok(stats) => HttpResponse::Ok().json(ApiResponse::success(stats)),
        Err(e) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
    }
}

pub async fn backup_database(
    _session: Session,
    database: web::Data<Database>,
) -> HttpResponse {
    let backup_path = format!("backups/flug_evidence_backup_{}.db", 
        chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    
    match database.backup(&backup_path).await {
        Ok(_) => HttpResponse::Ok().body(format!("Backup created: {}", backup_path)),
        Err(e) => HttpResponse::InternalServerError().body(format!("Backup failed: {}", e)),
    }
}

#[derive(Deserialize)]
pub struct AuditLogQuery {
    pub user_id: Option<String>,
    pub action_type: Option<String>,
    pub limit: Option<u32>,
}



// In admin_routes.rs
pub async fn cleanup_database(
    database: web::Data<Database>,
) -> impl Responder {
    match database.cleanup_old_records().await {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError().body(format!("Cleanup failed: {}", e)),
    }
}