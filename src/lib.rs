// src/lib.rs
pub mod auth;
pub mod media;
pub mod storj;
pub mod models;
pub mod routes;
pub mod email_service;
pub mod blockchain;
pub mod countries;
pub mod evidence_service;
pub mod settings_routes;   // ✅ Settings + POI + danger zone APIs
mod database;
pub mod target_routes;   // ✅ Target flag actions (pin / poi / watchlist / flag / takedown / notes / link-case)
pub mod intelligence_routes; // ✅ Intelligence subjects + cross-case intel flags
mod admin_routes;
pub mod face_client;     

// Re-export commonly used items
pub use database::Database;
pub use evidence_service::EvidenceService;

pub use models::{
    User, SessionUser, Evidence, EvidenceSummary, EvidenceForm, EvidenceUpdate,
    EmergencyLevel, IncidentType, VehicleType, EvidenceStatus,
    DashboardStats, CountyStats, IncidentTypeStats,
    ApiResponse, WalletConnection
};