// src/main.rs (RENDER-READY VERSION)
use actix_web::{web, App, HttpServer};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::{Key, SameSite};
use actix_files::Files;
use std::env;

mod auth;
mod storj;
mod media;
mod email_service;
mod models;
mod routes;
mod media_routes;
mod blockchain;
mod database;
mod admin_routes;
mod evidence_service;
mod countries;
mod settings_routes;   // ✅ Settings + POI + danger zone
mod target_routes;     // ✅ Target flag actions (pin / poi / watchlist / flag / takedown / notes / link-case)
mod intelligence_routes; // ✅ Intelligence subjects + cross-case intel flags (subjects table + intel_flags table)
mod face_client;    

use crate::evidence_service::EvidenceService;
use crate::database::Database;

#[derive(Debug, Clone)]
pub struct Config {
    pub storj_access_key: String,
    pub storj_secret_key: String,
    pub storj_endpoint: String,
    pub storj_sharing_key: String,   // linkshare key — baked into every file URL
    pub resend_api_key: String,
    pub resend_from_email: String,
    pub resend_from_name: String,
    pub database_path: String,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();
        
        Self {
            storj_access_key: env::var("STORJ_ACCESS_KEY")
                .unwrap_or_else(|_| "".to_string()),
            storj_secret_key: env::var("STORJ_SECRET_KEY")
                .unwrap_or_else(|_| "".to_string()),
            storj_endpoint: env::var("STORJ_ENDPOINT")
                .unwrap_or_else(|_| "https://gateway.storjshare.io".to_string()),
            storj_sharing_key: env::var("STORJ_SHARING_KEY")
                .unwrap_or_else(|_| "".to_string()),
            resend_api_key: env::var("RESEND_API_KEY")
                .unwrap_or_else(|_| "".to_string()),
            resend_from_email: env::var("RESEND_FROM_EMAIL")
                .unwrap_or_else(|_| "onboarding@resend.dev".to_string()),
            resend_from_name: env::var("RESEND_FROM_NAME")
                .unwrap_or_else(|_| "FLUG Evidence".to_string()),
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "data/flug_evidence.db".to_string()),
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    // ── Directory bootstrap ──────────────────────────────────────────────────
    for dir in &["data", "static", "static/js", "static/templates", "static/css"] {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| {
            println!("⚠️  Could not create directory {}: {}", dir, e);
        });
    }

    // ── Config ───────────────────────────────────────────────────────────────
    let config = Config::from_env();
    
    println!("=== Starting FLUG Evidence Server ===");
    println!("Storj Endpoint: {}", config.storj_endpoint);
    println!("Access key starts with: {}...", 
        &config.storj_access_key[..std::cmp::min(8, config.storj_access_key.len())]);
    println!("Bucket: crimebank");
    println!("Sharing key: {}",
        if config.storj_sharing_key.is_empty() {
            "⚠️  NOT SET — file URLs will use gateway fallback and break if credentials rotate".to_string()
        } else {
            format!("✅ Set ({}...)", &config.storj_sharing_key[..std::cmp::min(8, config.storj_sharing_key.len())])
        }
    );

    // ── Database ─────────────────────────────────────────────────────────────
    let database = match Database::new(&config.database_path) {
        Ok(db) => {
            println!("✅ Database initialized successfully");
            web::Data::new(db)
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize database: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Failed to initialize database: {}", e),
            ));
        }
    };

    // ── Target flags table ───────────────────────────────────────────────────
    // Must run after Database::new so the pool is live.
    target_routes::init_table(database.get_ref());

    // ── Intelligence subjects + intel_flags tables ───────────────────────────
    // Separate from target_routes — owns the `subjects` and `intel_flags` tables.
    intelligence_routes::init_tables(database.get_ref());

    // ── Auth service ─────────────────────────────────────────────────────────
    let auth_service = web::Data::new(auth::AuthService::new(database.get_ref().clone()));

    // ── Email service ─────────────────────────────────────────────────────────
    let email_service = match email_service::EmailService::new() {
        Ok(service) => {
            println!("✅ Resend Email service initialized successfully");
            Some(web::Data::new(service))
        }
        Err(e) => {
            println!("⚠️  Failed to initialize Resend Email service: {}", e);
            println!("⚠️  Using test mode — emails will be logged but not sent");
            Some(web::Data::new(email_service::EmailService::new_test_mode()))
        }
    };

    // ── Storj service ─────────────────────────────────────────────────────────
    let storj_service = match storj::StorjService::new(
        &config.storj_access_key,
        &config.storj_secret_key,
        &config.storj_endpoint,
        Some("crimebank"),
        if config.storj_sharing_key.is_empty() { None } else { Some(config.storj_sharing_key.as_str()) },
    ).await {
        Ok(service) => {
            println!("✅ Storj service initialized successfully");
            service
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize Storj service: {}", e);
            eprintln!("⚠️  Uploads will fail. Check your Storj credentials and endpoint.");
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Failed to initialize Storj: {}", e),
            ));
        }
    };

    println!("\n=== Testing Storj Credentials ===");
    match storj_service.test_credentials().await {
        Ok(_)  => println!("✅ Storj credentials test passed!"),
        Err(e) => {
            println!("⚠️  Storj credentials test failed: {}", e);
            println!("⚠️  Uploads will likely fail until this is fixed.");
        }
    }

    // ── Encodings bucket ─────────────────────────────────────────────────────
    println!("\n=== Initializing Encodings Bucket ===");
    match storj_service.ensure_encodings_bucket_exists().await {
        Ok(_)  => println!("✅ Encodings bucket  : Ready"),
        Err(e) => {
            println!("⚠️  Encodings bucket initialization failed: {}", e);
            println!("⚠️  Pickle uploads will fail until this is resolved.");
        }
    }

    // ── Country directory structure ───────────────────────────────────────────
    println!("\n=== Initializing Country Directory Structure ===");
    let all_countries = countries::get_all_countries();
    println!("📍 Total countries: {}", all_countries.len());

    match storj_service.initialize_country_directories(&all_countries).await {
        Ok(_)  => println!("✅ Country directories initialized successfully!"),
        Err(e) => {
            println!("⚠️  Country directory initialization had issues: {}", e);
            println!("⚠️  Directories will be created on-demand during uploads.");
        }
    }

    // ── Evidence service ──────────────────────────────────────────────────────
    let evidence_service = web::Data::new(
        EvidenceService::new(storj_service, database.get_ref().clone())
    );
    println!("✅ Evidence service initialized successfully");

        // ── Face sidecar health check ─────────────────────────────────────────────
    println!("\n=== Checking Face Service ===");
    let face_url = std::env::var("FACE_SERVICE_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());

    match reqwest::get(format!("{}/health", face_url)).await {
        Ok(resp) if resp.status().is_success() => {
            println!("✅ Face Service     : Ready ({})", face_url);
        }
        Ok(resp) => {
            println!("⚠️  Face Service     : Responded but not ready (HTTP {})", resp.status());
            println!("   → Run: cd services/face-service && pm2 start ecosystem.config.js");
        }
        Err(_) => {
            println!("⚠️  Face Service     : NOT RUNNING — face matching will be skipped");
            println!("   → Run: cd services/face-service && pm2 start ecosystem.config.js");
            println!("   → Uploads will still work, face matching resumes when service starts");
        }
    }

    // ── Session key ───────────────────────────────────────────────────────────
    let session_key = match env::var("SESSION_SECRET") {
        Ok(secret) if secret.len() >= 32 => {
            println!("✅ Using session secret from environment");
            Key::from(secret.as_bytes())
        }
        Ok(_) => {
            println!("⚠️  SESSION_SECRET too short (min 32 chars). Using generated key.");
            println!("⚠️  Sessions will reset on restart!");
            Key::generate()
        }
        Err(_) => {
            println!("⚠️  No SESSION_SECRET found. Using temporary key (resets on restart).");
            Key::generate()
        }
    };

    // ── Bind address ──────────────────────────────────────────────────────────
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    let bind_address = format!("0.0.0.0:{}", port);

    println!("\n✅ FLUG — Evidence Platform Ready");
    println!("🔐 Evidence Service : Ready");
    println!("📦 Storj Storage    : Ready");
    println!("🥒 Encodings Bucket : Ready");
    println!("🔒 Auth Service     : Ready");
    println!("🎯 Target Flags     : Ready");
    if email_service.is_some() { println!("📧 Email Service    : Ready"); }
    println!("🚀 Listening on     : {}", bind_address);
    println!("🔑 Session          : {}",
        if env::var("SESSION_SECRET").is_ok() { "Persistent" } else { "Temporary (resets on restart)" });

    // ── Startup maintenance ───────────────────────────────────────────────────
    if let Err(e) = database.get_ref().run_maintenance().await {
        println!("⚠️  Database maintenance failed: {}", e);
    }

    // ── Storj sharing key migration ───────────────────────────────────────────
    // Silently rewrites all stored URLs if STORJ_SHARING_KEY has changed since
    // the files were originally uploaded.  Safe to run every boot — exits
    // immediately if key matches or no linkshare URLs are present.
    if let Err(e) = database.get_ref()
        .migrate_storj_sharing_key(&config.storj_sharing_key).await
    {
        println!("⚠️  Storj URL migration failed: {}", e);
    }

    // ── HTTP server ───────────────────────────────────────────────────────────
    HttpServer::new(move || {
        let mut app = App::new()
            .app_data(auth_service.clone())
            .app_data(evidence_service.clone())
            .app_data(database.clone())
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    session_key.clone(),
                )
                .cookie_secure(false)
                .cookie_http_only(true)
                .cookie_same_site(SameSite::Lax)
                .cookie_name("flug_session".to_string())
                .session_lifecycle(
                    actix_session::config::PersistentSession::default()
                        .session_ttl(actix_web::cookie::time::Duration::hours(24))
                )
                .build()
            );

        if let Some(email_data) = &email_service {
            app = app.app_data(email_data.clone());
        }

        app
            // Static files
            .service(Files::new("/static", "./static").show_files_listing())

            // Auth routes
            .configure(routes::config)

            // Evidence / media routes
            .configure(media_routes::config)

            // Target flag action routes (pin / poi / watchlist / flag / takedown / notes / link-case)
            .configure(target_routes::config)

            // Intelligence subjects + cross-case intel flags
            .configure(intelligence_routes::config)

            // Settings, POI management, and danger zone
            .configure(settings_routes::config)

            // Admin routes
            .configure(|cfg| {
                cfg.route("/admin/audit-logs",    web::get().to(admin_routes::get_audit_logs))
                   .route("/admin/database-stats", web::get().to(admin_routes::get_database_stats))
                   .route("/admin/backup",         web::post().to(admin_routes::backup_database))
                   .route("/admin/cleanup",        web::post().to(admin_routes::cleanup_database));
            })
    })
    .bind(&bind_address)?
    .run()
    .await
}