// src/database.rs - FIXED VERSION
use std::path::Path;
// At the top of database.rs, add:
use std::collections::HashMap;
use rusqlite::{Connection, params, OptionalExtension, ToSql};
use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use anyhow::{Result, Context, anyhow};
use crate::models::*;
// Make sure you have this import at the top of database.rs
use crate::models::EvidenceLocationData; 
// In database.rs imports section, add:
use crate::models::{ChartData, MediaStorageStats, StorageSizeData};


#[derive(Debug, Clone)]
pub struct Database {
    pub pool: Pool<SqliteConnectionManager>,  // Add 'pub' here
}

impl Database {

    
    pub async fn run_maintenance(&self) -> Result<()> {
        println!("🛠️ Running database maintenance...");
        
        let conn = self.pool.get()?;
        
        // Run VACUUM to optimize database
        println!("   Running VACUUM...");
        conn.execute("VACUUM", [])?;
        
        // Analyze database for query optimization
        println!("   Running ANALYZE...");
        conn.execute("ANALYZE", [])?;
        
        // Clean up old sessions — guard against table not existing
        // (sessions table is optional; actix-session uses cookies, not DB sessions)
        println!("   Cleaning up old sessions...");
        let sessions_exist: bool = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
            [],
            |row| row.get::<_, i64>(0),
        ).unwrap_or(0) > 0;

        if sessions_exist {
            let one_day_ago = Utc::now().timestamp() - (24 * 60 * 60);
            conn.execute(
                "DELETE FROM sessions WHERE last_activity < ?1",
                params![one_day_ago],
            )?;
            println!("   Sessions cleaned up");
        } else {
            println!("   Sessions table not present — skipping");
        }
        
        // Clean up old verification tokens
        println!("   Cleaning up old verification tokens...");
        let one_day_ago = Utc::now().timestamp() - (24 * 60 * 60);
        conn.execute(
            "DELETE FROM verification_tokens WHERE created_at < ?1",
            params![one_day_ago],
        )?;
        
        // Get database stats
        let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let db_size_mb = (page_count * page_size) as f64 / (1024.0 * 1024.0);
        
        println!("✅ Database maintenance completed");
        println!("   Database size: {:.2} MB", db_size_mb);
        
        Ok(())
    }

    /// Silently checks whether the Storj sharing key embedded in every stored URL
    /// matches `current_key` (from STORJ_SHARING_KEY env).  If they differ, every
    /// affected URL is rewritten in a single transaction.
    ///
    /// Affected columns:
    ///   evidence_media  → storj_url, thumbnail_url
    ///   targets         → storj_url
    ///   evidence        → storj_urls  (JSON array — each element is rewritten)
    ///
    /// Call this from main.rs right after the Storj service is confirmed live,
    /// passing `config.storj_sharing_key`.
    pub async fn migrate_storj_sharing_key(&self, current_key: &str) -> Result<()> {
        if current_key.is_empty() {
            println!("🔑 [STORJ MIGRATE] No sharing key set — skipping URL migration.");
            return Ok(());
        }

        let conn = self.pool.get()?;

        // ── 1. Detect the key currently embedded in the DB ───────────────────
        // We sample the first non-empty storj_url from evidence_media.
        // Pattern: https://link.storjshare.io/raw/{key}/{bucket}/...
        let sample: Option<String> = conn
            .query_row(
                "SELECT storj_url FROM evidence_media \
                 WHERE storj_url LIKE 'https://link.storjshare.io/raw/%' \
                 LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;

        let db_key = match sample {
            None => {
                // No linkshare URLs in DB yet (fresh install or all gateway URLs)
                println!("🔑 [STORJ MIGRATE] No linkshare URLs found in DB — nothing to migrate.");
                return Ok(());
            }
            Some(url) => {
                // Extract the key segment: position between /raw/ and the next /
                url.strip_prefix("https://link.storjshare.io/raw/")
                    .and_then(|s| s.split('/').next())
                    .unwrap_or("")
                    .to_string()
            }
        };

        if db_key.is_empty() {
            println!("🔑 [STORJ MIGRATE] Could not parse existing key from URL — aborting.");
            return Ok(());
        }

        if db_key == current_key {
            println!("🔑 [STORJ MIGRATE] Sharing key unchanged ({}) — no migration needed.", 
                &current_key[..current_key.len().min(8)]);
            return Ok(());
        }

        // Keys differ — migrate everything
        println!("🔑 [STORJ MIGRATE] ⚠️  Sharing key mismatch detected!");
        println!("🔑 [STORJ MIGRATE]   DB key  : {}...", &db_key[..db_key.len().min(8)]);
        println!("🔑 [STORJ MIGRATE]   Env key : {}...", &current_key[..current_key.len().min(8)]);
        println!("🔑 [STORJ MIGRATE] Migrating all URLs...");

        let old_prefix = format!("https://link.storjshare.io/raw/{}/", db_key);
        let new_prefix = format!("https://link.storjshare.io/raw/{}/", current_key);

        // ── 2. Run all updates in a single transaction ────────────────────────
        conn.execute("BEGIN TRANSACTION", [])?;

        // evidence_media.storj_url
        let n1 = conn.execute(
            "UPDATE evidence_media SET storj_url = \
             REPLACE(storj_url, ?1, ?2) \
             WHERE storj_url LIKE 'https://link.storjshare.io/raw/%'",
            params![old_prefix, new_prefix],
        )?;

        // evidence_media.thumbnail_url  (may be NULL — REPLACE handles that safely)
        let n2 = conn.execute(
            "UPDATE evidence_media SET thumbnail_url = \
             REPLACE(thumbnail_url, ?1, ?2) \
             WHERE thumbnail_url LIKE 'https://link.storjshare.io/raw/%'",
            params![old_prefix, new_prefix],
        )?;

        // targets.storj_url
        let n3 = conn.execute(
            "UPDATE targets SET storj_url = \
             REPLACE(storj_url, ?1, ?2) \
             WHERE storj_url LIKE 'https://link.storjshare.io/raw/%'",
            params![old_prefix, new_prefix],
        )?;

        // evidence.storj_urls  — stored as a JSON string e.g.
        // '["https://link.storjshare.io/raw/KEY/bucket/path.mp4"]'
        // SQLite's REPLACE() works on the raw TEXT, which is safe here because
        // the key is a random alphanumeric token that won't appear in filenames.
        let n4 = conn.execute(
            "UPDATE evidence SET storj_urls = \
             REPLACE(storj_urls, ?1, ?2) \
             WHERE storj_urls LIKE '%link.storjshare.io/raw/%'",
            params![old_prefix, new_prefix],
        )?;

        conn.execute("COMMIT", [])?;

        let total = n1 + n2 + n3 + n4;
        println!("🔑 [STORJ MIGRATE] ✅ Migration complete — {} rows updated.", total);
        println!("🔑 [STORJ MIGRATE]   evidence_media.storj_url  : {} rows", n1);
        println!("🔑 [STORJ MIGRATE]   evidence_media.thumbnail  : {} rows", n2);
        println!("🔑 [STORJ MIGRATE]   targets.storj_url         : {} rows", n3);
        println!("🔑 [STORJ MIGRATE]   evidence.storj_urls       : {} rows", n4);

        Ok(())
    }

    pub async fn connect_wallet(&self, connection: &WalletConnection, user_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        let connection_id = format!("walletconn_{}", Uuid::new_v4());
        
        conn.execute(
            r#"
            INSERT OR REPLACE INTO wallet_connections 
            (id, user_id, wallet_address, chain, wallet_type, public_key, connected_at, last_used, is_active) 
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                connection_id,
                user_id,
                &connection.wallet_address,
                &connection.chain,
                &connection.wallet_type,
                &connection.public_key,
                connection.connected_at.timestamp(),
                connection.last_used.timestamp(),
                connection.is_active,
            ],
        )?;
        
        Ok(())
    }

    pub async fn get_user_wallets(&self, user_id: &str) -> Result<Vec<WalletConnection>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT wallet_address, chain, wallet_type, public_key, connected_at, last_used, is_active 
            FROM wallet_connections WHERE user_id = ?1 AND is_active = 1
            "#,
        )?;
        
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(WalletConnection {
                wallet_address: row.get(0)?,
                chain: row.get(1)?,
                wallet_type: row.get(2)?,
                public_key: row.get(3)?,
                connected_at: DateTime::from_timestamp(row.get::<_, i64>(4)?, 0)
                    .unwrap_or_else(|| Utc::now()),
                last_used: DateTime::from_timestamp(row.get::<_, i64>(5)?, 0)
                    .unwrap_or_else(|| Utc::now()),
                is_active: row.get::<_, i64>(6)? != 0,
            })
        })?;
        
        let mut wallets = Vec::new();
        for row in rows {
            wallets.push(row?);
        }
        Ok(wallets)
    }

    pub async fn get_wallet_connections(&self, email: &str) -> Result<Vec<WalletConnection>> {
        if let Some(user) = self.get_user_by_email(email).await? {
            self.get_user_wallets(&user.id).await
        } else {
            Ok(Vec::new())
        }
    }
    
    pub async fn disconnect_wallet(&self, user_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            "UPDATE wallet_connections SET is_active = 0 WHERE user_id = ?1",
            params![user_id],
        )?;
        
        Ok(())
    }
    
    pub async fn remove_user(&self, email: &str) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute("BEGIN TRANSACTION", [])?;
        conn.execute("DELETE FROM users WHERE email = ?1", params![email])?;
        conn.execute("DELETE FROM wallet_connections WHERE user_id IN (SELECT id FROM users WHERE email = ?1)", params![email])?;
        conn.execute("DELETE FROM verification_tokens WHERE user_email = ?1", params![email])?;
        conn.execute("COMMIT", [])?;
        
        Ok(())
    }
    
    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, email, password_hash, is_verified, verification_token, 
            wallet_address, wallet_type, wallet_chain, public_key, created_at, updated_at,
            account_type, business_name, geo_latitude, geo_longitude, is_profile_complete,
            phone_number, county, id_number
            FROM users
            "#,
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                email: row.get(1)?,
                password_hash: row.get(2)?,
                is_verified: row.get::<_, i64>(3)? != 0,
                verification_token: row.get(4)?,
                wallet_address: row.get(5)?,
                wallet_type: row.get(6)?,
                wallet_chain: row.get(7)?,
                public_key: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                account_type: row.get(11)?,
                business_name: row.get(12)?,
                geo_latitude: row.get(13)?,
                geo_longitude: row.get(14)?,
                is_profile_complete: row.get::<_, i64>(15)? != 0,
                phone_number: row.get(16)?,
                county: row.get(17)?,
                id_number: row.get(18)?,
            })
        })?;
        
        let mut users = Vec::new();
        for row in rows {
            users.push(row?);
        }
        Ok(users)
    }
    
    pub async fn find_user_by_wallet(&self, wallet_address: &str) -> Result<Option<User>> {
        let conn = self.pool.get()?;
        
        // Try exact match first
        let user = conn.query_row(
            r#"
            SELECT id, email, password_hash, is_verified, verification_token, 
            wallet_address, wallet_type, wallet_chain, public_key, created_at, updated_at,
            account_type, business_name, geo_latitude, geo_longitude, is_profile_complete,
            phone_number, county, id_number
            FROM users WHERE wallet_address = ?1 COLLATE NOCASE
            "#,
            params![wallet_address],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    password_hash: row.get(2)?,
                    is_verified: row.get::<_, i64>(3)? != 0,
                    verification_token: row.get(4)?,
                    wallet_address: row.get(5)?,
                    wallet_type: row.get(6)?,
                    wallet_chain: row.get(7)?,
                    public_key: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    account_type: row.get(11)?,
                    business_name: row.get(12)?,
                    geo_latitude: row.get(13)?,
                    geo_longitude: row.get(14)?,
                    is_profile_complete: row.get::<_, i64>(15)? != 0,
                    phone_number: row.get(16)?,
                    county: row.get(17)?,
                    id_number: row.get(18)?,
                })
            },
        ).optional()?;
        
        if user.is_some() {
            return Ok(user);
        }
        
        // If not found by exact match, try case-insensitive
        let user = conn.query_row(
            r#"
            SELECT id, email, password_hash, is_verified, verification_token, 
            wallet_address, wallet_type, wallet_chain, public_key, created_at, updated_at,
            account_type, business_name, geo_latitude, geo_longitude, is_profile_complete,
            phone_number, county, id_number
            FROM users WHERE LOWER(wallet_address) = LOWER(?1)
            "#,
            params![wallet_address],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    password_hash: row.get(2)?,
                    is_verified: row.get::<_, i64>(3)? != 0,
                    verification_token: row.get(4)?,
                    wallet_address: row.get(5)?,
                    wallet_type: row.get(6)?,
                    wallet_chain: row.get(7)?,
                    public_key: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                    account_type: row.get(11)?,
                    business_name: row.get(12)?,
                    geo_latitude: row.get(13)?,
                    geo_longitude: row.get(14)?,
                    is_profile_complete: row.get::<_, i64>(15)? != 0,
                    phone_number: row.get(16)?,
                    county: row.get(17)?,
                    id_number: row.get(18)?,
                })
            },
        ).optional()?;
        
        Ok(user)
    }
    
    pub fn new(db_path: &str) -> Result<Self> {
        println!("📊 DATABASE: Initializing database at: {}", db_path);
        
        if let Some(parent) = Path::new(db_path).parent() {
            std::fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {:?}", parent))?;
        }
        
        if !Path::new(db_path).exists() {
            println!("📊 DATABASE: Creating new database file...");
            std::fs::File::create(db_path)
                .context(format!("Failed to create database file: {}", db_path))?;
        }
        
        let metadata = std::fs::metadata(db_path)
            .context(format!("Failed to get metadata for: {}", db_path))?;
        println!("📊 DATABASE: File permissions: {:?}", metadata.permissions());
        
        let manager = SqliteConnectionManager::file(db_path)
            .with_flags(rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | 
                       rusqlite::OpenFlags::SQLITE_OPEN_CREATE);
        
        let pool = Pool::builder()
            .max_size(5)
            .connection_timeout(std::time::Duration::from_secs(5))
            .build(manager)
            .context("Failed to create database connection pool")?;
        
        let db = Self { pool };
        
        println!("📊 DATABASE: Testing database connection...");
        let _test_conn = db.pool.get()
            .context("Failed to get test connection from pool")?;
        
        db.initialize_tables()
            .context("Failed to initialize database tables")?;
        
        println!("✅ DATABASE: Database initialized successfully");
        
        Ok(db)
    }
    
    fn initialize_tables(&self) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute("PRAGMA foreign_keys = ON;", [])?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        
        // USERS TABLE - Updated with Kenya fields
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT,
                is_verified BOOLEAN NOT NULL DEFAULT 0,
                verification_token TEXT,
                wallet_address TEXT,
                wallet_type TEXT,
                wallet_chain TEXT,
                public_key TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                last_login_at INTEGER,
                login_count INTEGER DEFAULT 0,
                account_type TEXT,
                business_name TEXT,
                geo_latitude REAL,
                geo_longitude REAL,
                is_profile_complete BOOLEAN DEFAULT 0,
                phone_number TEXT,
                county TEXT,
                id_number TEXT,
                metadata TEXT DEFAULT '{}'
            );
            "#,
            [],
        )?;
        
        // WALLET_CONNECTIONS TABLE
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS wallet_connections (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                chain TEXT NOT NULL,
                wallet_type TEXT NOT NULL,
                public_key TEXT,
                connected_at INTEGER NOT NULL,
                last_used INTEGER NOT NULL,
                is_active BOOLEAN DEFAULT 1,
                connection_count INTEGER DEFAULT 1,
                metadata TEXT DEFAULT '{}',
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(user_id, wallet_address)
            );
            "#,
            [],
        )?;
        
        // VERIFICATION_TOKENS TABLE
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS verification_tokens (
                token TEXT PRIMARY KEY,
                user_email TEXT NOT NULL,
                token_type TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                used_at INTEGER,
                FOREIGN KEY (user_email) REFERENCES users(email) ON DELETE CASCADE
            );
            "#,
            [],
        )?;
        
        // AUDIT_LOG TABLE
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS audit_log (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                action_type TEXT NOT NULL,
                action_target TEXT NOT NULL,
                target_id TEXT,
                ip_address TEXT,
                user_agent TEXT,
                details TEXT NOT NULL,
                created_at INTEGER NOT NULL
            );
            "#,
            [],
        )?;
        
        // EVIDENCE TABLE (CRITICAL - REPLACES CONTENT)
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS evidence (
                id TEXT PRIMARY KEY,
                evidence_number TEXT UNIQUE NOT NULL,
                emergency_level TEXT NOT NULL CHECK(emergency_level IN ('red','orange','yellow','blue')),
                incident_type TEXT NOT NULL,
                sub_type TEXT,
                
                -- Time fields
                incident_time INTEGER NOT NULL,
                report_time INTEGER NOT NULL,
                
                -- Location (MANDATORY for Kenya)
                county TEXT NOT NULL,
                constituency TEXT,
                ward TEXT,
                latitude REAL NOT NULL,
                longitude REAL NOT NULL,
                landmark TEXT,
                
                -- Vehicle details (if hit & run)
                vehicle_registration TEXT,
                vehicle_color TEXT,
                vehicle_type TEXT,
                sacco_name TEXT,
                vehicle_description TEXT,
                
                -- Description
                title TEXT NOT NULL,
                description TEXT NOT NULL,
                injuries TEXT,
                property_damage TEXT,
                suspect_description TEXT,
                
                -- Uploader info
                uploader_id TEXT NOT NULL,
                uploader_email TEXT NOT NULL,
                uploader_phone TEXT,
                
                -- Media info
                media_count INTEGER DEFAULT 0,
                evidence_quality INTEGER DEFAULT 1,
                file_size_bytes INTEGER NOT NULL,
                mime_types TEXT DEFAULT '[]',
                hash_values TEXT DEFAULT '[]',
                
                -- Police integration
                reported_to_police BOOLEAN DEFAULT 0,
                police_case_id TEXT,
                police_station TEXT,
                report_date INTEGER,
                
                -- Blockchain
                wallet_signature TEXT,
                wallet_address TEXT,
                signature_timestamp INTEGER,
                
                -- Status
                status TEXT DEFAULT 'submitted' CHECK(status IN ('draft','submitted','reported','under_review','archived','rejected')),
                needs_attention BOOLEAN DEFAULT 0,
                is_anonymous BOOLEAN DEFAULT 0,
                chain_of_custody TEXT DEFAULT '[]',
                
                -- Storage
                storj_urls TEXT DEFAULT '[]',
                storj_bucket TEXT,
                
                -- Timestamps
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                reviewed_at INTEGER
            );
            "#,
            [],
        )?;
        
        // EVIDENCE_MEDIA TABLE
        conn.execute(
            r#"
        CREATE TABLE IF NOT EXISTS evidence_media (
            id TEXT PRIMARY KEY,
            evidence_id TEXT NOT NULL,
            filename TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            duration_seconds INTEGER,
            thumbnail_url TEXT,
            storj_url TEXT NOT NULL,
            storj_key TEXT NOT NULL,
            hash TEXT NOT NULL,
            quality_rating INTEGER DEFAULT 3,
            description TEXT,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (evidence_id) REFERENCES evidence(id) ON DELETE CASCADE
        );
            "#,
            [],
        )?;
        
        // EVIDENCE_SIGNATURES TABLE
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS evidence_signatures (
                id TEXT PRIMARY KEY,
                evidence_id TEXT NOT NULL,
                wallet_address TEXT NOT NULL,
                signature TEXT NOT NULL,
                signed_hash TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                chain TEXT NOT NULL,
                transaction_id TEXT,
                FOREIGN KEY (evidence_id) REFERENCES evidence(id) ON DELETE CASCADE
            );
            "#,
            [],
        )?;


        // TARGETS TABLE
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS targets (
                id TEXT PRIMARY KEY,
                evidence_id TEXT NOT NULL,
                target_number INTEGER NOT NULL,
                filename TEXT NOT NULL,
                mime_type TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                description TEXT,
                category TEXT CHECK(category IN ('person', 'vehicle', 'object', 'location', 'other')),
                confidence_score INTEGER DEFAULT 50,
                storj_url TEXT NOT NULL,
                storj_key TEXT NOT NULL,
                hash TEXT NOT NULL,
                phash TEXT,                          -- perceptual hash (16-char hex) for fallback matching
                auto_generated INTEGER DEFAULT 0,    -- 1 = auto-detected face, not user-selected
                created_at INTEGER NOT NULL,
                created_by TEXT NOT NULL,
                FOREIGN KEY (evidence_id) REFERENCES evidence(id) ON DELETE CASCADE,
                UNIQUE(evidence_id, target_number)
            );
            "#,
            [],
        )?;


        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_targets_evidence_id ON targets(evidence_id);
            "#,
            [],
        )?;

        // FACE_ENCODINGS TABLE — stores 128-dim face descriptors for fuzzy matching
        // descriptor BLOB = 128 × f32 (little-endian) = 512 bytes per face
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS face_encodings (
                id              TEXT PRIMARY KEY,
                target_id       TEXT NOT NULL,
                evidence_id     TEXT NOT NULL,
                face_index      INTEGER NOT NULL DEFAULT 0,  -- 0 = first/largest face in image
                descriptor      BLOB NOT NULL,               -- 128 × f32 LE = 512 bytes
                detection_score REAL NOT NULL DEFAULT 0.0,   -- face-api.js detection confidence (0.0–1.0)
                phash           TEXT,                        -- 16-char hex pHash (fallback for non-face targets)
                auto_generated  INTEGER NOT NULL DEFAULT 0,  -- 1 = auto-split from multi-face upload
                created_at      INTEGER NOT NULL,
                FOREIGN KEY (target_id)   REFERENCES targets(id)  ON DELETE CASCADE,
                FOREIGN KEY (evidence_id) REFERENCES evidence(id) ON DELETE CASCADE
            );
            "#,
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_face_encodings_target   ON face_encodings(target_id);",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_face_encodings_evidence ON face_encodings(evidence_id);",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_face_encodings_auto     ON face_encodings(auto_generated);",
            [],
        )?;

        // Create linked_evidence table for connecting related cases
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS linked_evidence (
                id TEXT PRIMARY KEY,
                evidence_id_1 TEXT NOT NULL,
                evidence_id_2 TEXT NOT NULL,
                link_type TEXT NOT NULL,
                link_reason TEXT,
                matched_target_hash TEXT,
                confidence_score INTEGER DEFAULT 100,
                created_at INTEGER NOT NULL,
                created_by TEXT,
                FOREIGN KEY (evidence_id_1) REFERENCES evidence(id) ON DELETE CASCADE,
                FOREIGN KEY (evidence_id_2) REFERENCES evidence(id) ON DELETE CASCADE,
                UNIQUE(evidence_id_1, evidence_id_2)
            );
            "#,
            [],
        )?;

        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_linked_evidence_1 ON linked_evidence(evidence_id_1);
            "#,
            [],
        )?;
        
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_linked_evidence_2 ON linked_evidence(evidence_id_2);
            "#,
            [],
        )?;
        
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_linked_evidence_hash ON linked_evidence(matched_target_hash);
            "#,
            [],
        )?;

        // linked_cases: write table used by the API (api_create/delete_linked_case).
        // linked_evidence is the legacy read-only table.  Both are queried via
        // get_linked_evidence() which UNIONs them.
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS linked_cases (
                id TEXT PRIMARY KEY,
                evidence_id_1 TEXT NOT NULL,
                evidence_id_2 TEXT NOT NULL,
                link_type TEXT NOT NULL DEFAULT 'manual',
                link_reason TEXT,
                matched_target_hash TEXT,
                confidence_score INTEGER DEFAULT 50,
                created_at TEXT NOT NULL,
                created_by TEXT,
                notes TEXT,
                FOREIGN KEY (evidence_id_1) REFERENCES evidence(id) ON DELETE CASCADE,
                FOREIGN KEY (evidence_id_2) REFERENCES evidence(id) ON DELETE CASCADE
            );
            "#,
            [],
        )?;
        conn.execute(
            r#"CREATE INDEX IF NOT EXISTS idx_linked_cases_1 ON linked_cases(evidence_id_1);"#,
            [],
        )?;
        conn.execute(
            r#"CREATE INDEX IF NOT EXISTS idx_linked_cases_2 ON linked_cases(evidence_id_2);"#,
            [],
        )?;

        // Create notifications table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS notifications (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                notification_type TEXT NOT NULL,
                title TEXT NOT NULL,
                message TEXT NOT NULL,
                evidence_id TEXT,
                linked_evidence_id TEXT,
                target_hash TEXT,
                is_read INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                read_at INTEGER,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );
            "#,
            [],
        )?;

        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_notifications_user ON notifications(user_id);
            "#,
            [],
        )?;
        
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_notifications_read ON notifications(is_read);
            "#,
            [],
        )?;
        
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_notifications_created ON notifications(created_at);
            "#,
            [],
        )?;
        
        conn.execute(
            r#"
            CREATE INDEX IF NOT EXISTS idx_targets_hash ON targets(hash);
            "#,
            [],
        )?;

        // PLATFORM_SETTINGS — generic JSON key-value store per user
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS platform_settings (
                id          TEXT PRIMARY KEY,
                user_id     TEXT NOT NULL,
                key         TEXT NOT NULL,
                value_json  TEXT NOT NULL,
                updated_at  INTEGER NOT NULL,
                UNIQUE(user_id, key)
            );
            "#,
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_settings_user ON platform_settings(user_id);",
            [],
        )?;

        // PERSONS_OF_INTEREST — pinned POI profiles
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS persons_of_interest (
                id               TEXT PRIMARY KEY,
                poi_number       TEXT UNIQUE NOT NULL,
                display_name     TEXT NOT NULL,
                category         TEXT NOT NULL CHECK(category IN ('person','vehicle','unknown')),
                status           TEXT NOT NULL DEFAULT 'watching'
                                     CHECK(status IN ('watching','active','resolved','archived')),
                linked_cases     INTEGER DEFAULT 0,
                linked_evidence  TEXT DEFAULT '[]',
                notes            TEXT,
                pinned_by        TEXT NOT NULL,
                created_at       INTEGER NOT NULL,
                last_seen_at     INTEGER,
                resolved_at      INTEGER,
                FOREIGN KEY (pinned_by) REFERENCES users(id) ON DELETE CASCADE
            );
            "#,
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_poi_user   ON persons_of_interest(pinned_by);",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_poi_status ON persons_of_interest(status);",
            [],
        )?;

        println!("📊 DATABASE: All tables and indexes created");
        
        Ok(())
    }
    
    pub async fn create_user(&self, user: &User) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            r#"
            INSERT INTO users (
                id, email, password_hash, is_verified, verification_token,
                wallet_address, wallet_type, wallet_chain, public_key,
                created_at, updated_at,
                account_type, business_name, geo_latitude, geo_longitude,
                is_profile_complete, phone_number, county, id_number
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                user.id,
                user.email,
                user.password_hash,
                user.is_verified,
                user.verification_token,
                user.wallet_address,
                user.wallet_type,
                user.wallet_chain,
                user.public_key,
                user.created_at,
                user.updated_at,
                user.account_type,
                user.business_name,
                user.geo_latitude,
                user.geo_longitude,
                user.is_profile_complete,
                user.phone_number,
                user.county,
                user.id_number,
            ],
        )?;
        
        self.log_audit(
            Some(&user.id),
            "user_created",
            "user",
            Some(&user.id),
            &format!("User created: {}", user.email),
            None,
        ).await?;
        
        Ok(())
    }
    
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                id, email, password_hash, is_verified, verification_token,
                wallet_address, wallet_type, wallet_chain, public_key,
                created_at, updated_at,
                account_type, business_name, geo_latitude, geo_longitude,
                is_profile_complete, phone_number, county, id_number
            FROM users WHERE email = ?
            "#,
        )?;
        
        let user = stmt.query_row([email], |row| {
            Ok(User {
                id: row.get(0)?,
                email: row.get(1)?,
                password_hash: row.get(2)?,
                is_verified: row.get::<_, i64>(3)? != 0,
                verification_token: row.get(4)?,
                wallet_address: row.get(5)?,
                wallet_type: row.get(6)?,
                wallet_chain: row.get(7)?,
                public_key: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
                account_type: row.get(11)?,
                business_name: row.get(12)?,
                geo_latitude: row.get(13)?,
                geo_longitude: row.get(14)?,
                is_profile_complete: row.get::<_, i64>(15)? != 0,
                phone_number: row.get(16)?,
                county: row.get(17)?,
                id_number: row.get(18)?,
            })
        }).optional()?;
        
        Ok(user)
    }
    
    pub async fn update_user(&self, user: &User) -> Result<()> {
    let conn = self.pool.get()?;
    
    println!("💾 DATABASE_UPDATE_USER: Updating user {}", user.id);
    println!("   Email: {}", user.email);
    println!("   Phone: {:?}", user.phone_number);
    println!("   County: {:?}", user.county);
    
    let rows_affected = conn.execute(
        r#"
        UPDATE users SET
            email = ?,
            password_hash = ?,
            is_verified = ?,
            verification_token = ?,
            wallet_address = ?,
            wallet_type = ?,
            wallet_chain = ?,
            public_key = ?,
            updated_at = ?,
            account_type = ?,
            business_name = ?,
            geo_latitude = ?,
            geo_longitude = ?,
            is_profile_complete = ?,
            phone_number = ?,
            county = ?,
            id_number = ?
        WHERE id = ?
        "#,
        params![
            user.email,           // 1
            user.password_hash,   // 2
            user.is_verified,     // 3
            user.verification_token, // 4
            user.wallet_address,  // 5
            user.wallet_type,     // 6
            user.wallet_chain,    // 7
            user.public_key,      // 8
            user.updated_at,      // 9
            user.account_type,    // 10
            user.business_name,   // 11
            user.geo_latitude,    // 12
            user.geo_longitude,   // 13
            user.is_profile_complete, // 14
            user.phone_number,    // 15
            user.county,          // 16
            user.id_number,       // 17
            user.id               // 18
        ],
    )?;
    
    println!("✅ DATABASE_UPDATE_USER: Updated {} rows for user {}", rows_affected, user.id);
    
    self.log_audit(
        Some(&user.id),
        "user_updated",
        "user",
        Some(&user.id),
        &format!("User updated: {} - County: {:?}", user.email, user.county),
        None,
    ).await?;
    
    Ok(())
}
    
    pub async fn update_user_login(&self, user_id: &str, ip_address: Option<&str>) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now().timestamp() as u64;
        
        conn.execute(
            r#"
            UPDATE users SET
                last_login_at = ?,
                login_count = login_count + 1,
                updated_at = ?
            WHERE id = ?
            "#,
            params![now, now, user_id],
        )?;
        
        self.log_audit(
            Some(user_id),
            "user_login",
            "user",
            Some(user_id),
            "User logged in",
            ip_address,
        ).await?;
        
        Ok(())
    }
    
    pub async fn store_verification_token(
        &self,
        token: &str,
        user_email: &str,
        token_type: &str,
        expires_in_hours: u32,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now().timestamp() as u64;
        let expires_at = now + (expires_in_hours as u64 * 3600);
        
        conn.execute(
            r#"
            INSERT INTO verification_tokens (
                token, user_email, token_type, created_at, expires_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
            params![token, user_email, token_type, now, expires_at],
        )?;
        
        Ok(())
    }
    
    pub async fn verify_token(&self, token: &str, token_type: &str) -> Result<Option<String>> {
        let conn = self.pool.get()?;
        let now = Utc::now().timestamp() as u64;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT user_email, expires_at, used_at 
            FROM verification_tokens 
            WHERE token = ? AND token_type = ?
            "#,
        )?;
        
        let result = stmt.query_row([token, token_type], |row| {
            let email: String = row.get(0)?;
            let expires_at: u64 = row.get(1)?;
            let used_at: Option<u64> = row.get(2)?;
            
            Ok((email, expires_at, used_at))
        }).optional()?;
        
        if let Some((email, expires_at, used_at)) = result {
            if now > expires_at {
                conn.execute(
                    "DELETE FROM verification_tokens WHERE token = ?",
                    [token],
                )?;
                return Ok(None);
            }
            
            if used_at.is_some() {
                return Ok(None);
            }
            
            conn.execute(
                "UPDATE verification_tokens SET used_at = ? WHERE token = ?",
                params![now, token],
            )?;
            
            self.log_audit(
                None,
                "token_verified",
                "token",
                Some(token),
                &format!("Token verified: {}", token_type),
                None,
            ).await?;
            
            Ok(Some(email))
        } else {
            Ok(None)
        }
    }
    
    pub async fn log_audit(
        &self,
        user_id: Option<&str>,
        action_type: &str,
        action_target: &str,
        target_id: Option<&str>,
        details: &str,
        ip_address: Option<&str>,
    ) -> Result<()> {
        let conn = self.pool.get()?;
        let now = Utc::now().timestamp() as u64;
        let audit_id = format!("audit_{}", Uuid::new_v4());
        
        let user_agent = std::env::var("USER_AGENT").ok();
        
        conn.execute(
            r#"
            INSERT INTO audit_log (
                id, user_id, action_type, action_target, target_id,
                ip_address, user_agent, details, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                audit_id,
                user_id,
                action_type,
                action_target,
                target_id,
                ip_address,
                user_agent,
                details,
                now
            ],
        )?;
        
        Ok(())
    }
    
    pub async fn get_audit_logs(
        &self,
        user_id: Option<&str>,
        action_type: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AuditLogEntry>> {
        let conn = self.pool.get()?;
        
        let mut where_clauses = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();
        
        if let Some(uid) = user_id {
            where_clauses.push("user_id = ?".to_string());
            params.push(Box::new(uid.to_string()));
        }
        
        if let Some(action) = action_type {
            where_clauses.push("action_type = ?".to_string());
            params.push(Box::new(action.to_string()));
        }
        
        let where_clause = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        
        let mut stmt = conn.prepare(&format!(
            r#"
            SELECT 
                id, user_id, action_type, action_target, target_id,
                ip_address, user_agent, details, created_at
            FROM audit_log 
            {}
            ORDER BY created_at DESC
            LIMIT ?
            "#,
            where_clause
        ))?;
        
        params.push(Box::new(limit as i64));
        
        let rows = stmt.query_map(
            params.iter().map(|p| &**p).collect::<Vec<_>>().as_slice(),
            |row| {
                let created_at: i64 = row.get(8)?;
                
                Ok(AuditLogEntry {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    action_type: row.get(2)?,
                    action_target: row.get(3)?,
                    target_id: row.get(4)?,
                    ip_address: row.get(5)?,
                    user_agent: row.get(6)?,
                    details: row.get(7)?,
                    created_at: DateTime::from_timestamp(created_at, 0)
                        .unwrap_or_else(Utc::now),
                })
            },
        )?;
        
        let mut logs = Vec::new();
        for row in rows {
            logs.push(row?);
        }
        
        Ok(logs)
    }
    
    pub async fn get_database_stats(&self) -> Result<DatabaseStats> {
        let conn = self.pool.get()?;
        
        let total_users: i64 = conn.query_row(
            "SELECT COUNT(*) FROM users", 
            [], 
            |row| row.get(0)
        )?;
        
        let total_evidence: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE status != 'rejected'", 
            [], 
            |row| row.get(0)
        )?;
        
        let urgent_evidence: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE emergency_level IN ('red', 'orange')", 
            [], 
            |row| row.get(0)
        )?;
        
        let reported_evidence: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE reported_to_police = 1", 
            [], 
            |row| row.get(0)
        )?;
        
        let total_wallet_connections: i64 = conn.query_row(
            "SELECT COUNT(*) FROM wallet_connections WHERE is_active = 1", 
            [], 
            |row| row.get(0)
        )?;
        
        let total_audit_logs: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log", 
            [], 
            |row| row.get(0)
        )?;
        
        let db_path = "data/flug_evidence.db";
        let database_size_bytes = std::fs::metadata(db_path)
            .map(|m| m.len())
            .unwrap_or(0);
        
        Ok(DatabaseStats {
            total_users,
            total_evidence,
            urgent_evidence,
            reported_evidence,
            total_wallet_connections,
            total_audit_logs,
            database_size_bytes,
        })
    }
    
    pub async fn backup(&self, backup_path: &str) -> Result<()> {
        println!("📊 DATABASE: Creating backup at: {}", backup_path);
        
        let db_path = "data/flug_evidence.db";
        
        if let Some(parent) = std::path::Path::new(backup_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        std::fs::copy(db_path, backup_path)
            .context(format!("Failed to copy database from {} to {}", db_path, backup_path))?;
        
        println!("✅ DATABASE: Backup created successfully at: {}", backup_path);
        Ok(())
    }

    // ==================== EVIDENCE METHODS ====================
    
    pub async fn create_evidence(&self, evidence: &Evidence) -> Result<()> {
        let conn = self.pool.get()?;
        
        println!("💾 DATABASE_CREATE_EVIDENCE: Creating evidence: {}", evidence.id);
        println!("   Evidence Number: {}", evidence.evidence_number);
        println!("   Media files to store: {}", evidence.media_files.len());
        
        // Start transaction
        conn.execute("BEGIN TRANSACTION", [])?;
        
        // Serialize JSON fields
        let mime_types_json = serde_json::to_string(&evidence.mime_types)
            .context("Failed to serialize mime_types")?;
        let hash_values_json = serde_json::to_string(&evidence.hash_values)
            .context("Failed to serialize hash_values")?;
        let storj_urls_json = serde_json::to_string(&evidence.storj_urls)
            .context("Failed to serialize storj_urls")?;
        let chain_of_custody_json = serde_json::to_string(&evidence.chain_of_custody)
            .context("Failed to serialize chain_of_custody")?;
        
        // Insert evidence record
        conn.execute(
            r#"
            INSERT INTO evidence (
                id, evidence_number, emergency_level, incident_type, sub_type,
                incident_time, report_time, county, constituency, ward,
                latitude, longitude, landmark, vehicle_registration, vehicle_color,
                vehicle_type, sacco_name, vehicle_description, title, description,
                injuries, property_damage, suspect_description, uploader_id,
                uploader_email, uploader_phone, media_count, evidence_quality,
                file_size_bytes, mime_types, hash_values, reported_to_police,
                police_case_id, police_station, report_date, wallet_signature,
                wallet_address, signature_timestamp, status, needs_attention,
                is_anonymous, chain_of_custody, storj_urls, storj_bucket,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                    ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?,
                    ?, ?, ?, ?, ?, ?)
            "#,
            params![
                evidence.id,
                evidence.evidence_number,
                match evidence.emergency_level {
                    EmergencyLevel::Red => "red",
                    EmergencyLevel::Orange => "orange",
                    EmergencyLevel::Yellow => "yellow",
                    EmergencyLevel::Blue => "blue",
                },
                match evidence.incident_type {
                    IncidentType::HitAndRun => "HitAndRun",
                    IncidentType::Assault => "Assault",
                    IncidentType::ThreatToLife => "ThreatToLife",
                    IncidentType::PropertyDamage => "PropertyDamage",
                    IncidentType::Theft => "Theft",
                    IncidentType::Other => "Other",
                },
                evidence.sub_type,
                evidence.incident_time.timestamp(),
                evidence.report_time.timestamp(),
                evidence.location.county,
                evidence.location.constituency,
                evidence.location.ward,
                evidence.location.latitude,
                evidence.location.longitude,
                evidence.location.landmark,
                evidence.vehicle_details.as_ref().and_then(|v| v.registration.clone()),
                evidence.vehicle_details.as_ref().and_then(|v| v.color.clone()),
                evidence.vehicle_details.as_ref().and_then(|v| v.vehicle_type.as_ref().map(|vt| match vt {
                    VehicleType::Matatu => "Matatu",
                    VehicleType::BodaBoda => "BodaBoda",
                    VehicleType::Private => "Private",
                    VehicleType::PSV => "PSV",
                    VehicleType::Lorry => "Lorry",
                    VehicleType::Taxi => "Taxi",
                    VehicleType::Unknown => "Unknown",
                })),
                evidence.vehicle_details.as_ref().and_then(|v| v.sacco_name.clone()),
                evidence.vehicle_details.as_ref().and_then(|v| v.description.clone()),
                evidence.title,
                evidence.description,
                evidence.injuries,
                evidence.property_damage,
                evidence.suspect_description,
                evidence.uploader_id,
                evidence.uploader_email,
                evidence.uploader_phone,
                evidence.media_files.len() as i32,
                evidence.evidence_quality,
                evidence.file_size_bytes as i64,
                mime_types_json,
                hash_values_json,
                evidence.reported_to_police,
                evidence.police_case_id,
                evidence.police_station,
                evidence.report_date.map(|d| d.timestamp()),
                evidence.wallet_signature,
                evidence.wallet_address,
                evidence.signature_timestamp.map(|d| d.timestamp()),
                match evidence.status {
                    EvidenceStatus::Draft => "draft",
                    EvidenceStatus::Submitted => "submitted",
                    EvidenceStatus::Reported => "reported",
                    EvidenceStatus::UnderReview => "under_review",
                    EvidenceStatus::Archived => "archived",
                    EvidenceStatus::Rejected => "rejected",
                },
                evidence.needs_attention,
                evidence.is_anonymous,
                chain_of_custody_json,
                storj_urls_json,
                evidence.storj_bucket,
                evidence.created_at.timestamp(),
                evidence.updated_at.timestamp(),
            ],
        )?;
        
        println!("✅ DATABASE_CREATE_EVIDENCE: Evidence record created");
        
        // Store media files in evidence_media table
        let mut media_success = 0;
        let mut media_failed = 0;
        
        for (index, media) in evidence.media_files.iter().enumerate() {
            println!("📁 DATABASE_CREATE_EVIDENCE: Storing media file {}: {}", index + 1, media.filename);
            
            // Validate required fields
            if media.storj_url.is_empty() {
                println!("⚠️ DATABASE_CREATE_EVIDENCE: Media file {} has empty storj_url, skipping", media.filename);
                media_failed += 1;
                continue;
            }
            
            if media.storj_key.is_empty() {
                println!("⚠️ DATABASE_CREATE_EVIDENCE: Media file {} has empty storj_key, skipping", media.filename);
                media_failed += 1;
                continue;
            }
            
            if media.hash.is_empty() {
                println!("⚠️ DATABASE_CREATE_EVIDENCE: Media file {} has empty hash, skipping", media.filename);
                media_failed += 1;
                continue;
            }
            
            match conn.execute(
                r#"
                INSERT INTO evidence_media (
                    id, evidence_id, filename, mime_type, file_size,
                    duration_seconds, thumbnail_url, storj_url, storj_key,
                    hash, quality_rating, description, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    media.id,
                    evidence.id,
                    media.filename,
                    media.mime_type,
                    media.file_size as i64,
                    media.duration_seconds,
                    media.thumbnail_url,
                    media.storj_url,
                    media.storj_key,
                    media.hash,
                    media.quality_rating,
                    media.description,
                    evidence.created_at.timestamp(),
                ],
            ) {
                Ok(_) => {
                    media_success += 1;
                    println!("✅ DATABASE_CREATE_EVIDENCE: Media file {} stored: {}", index + 1, media.filename);
                    println!("   MIME Type: {}", media.mime_type);
                    println!("   Size: {} bytes", media.file_size);
                    println!("   Storj URL: {}", media.storj_url);
                    println!("   Hash: {}", &media.hash[..16]); // Show first 16 chars of hash
                }
                Err(e) => {
                    media_failed += 1;
                    println!("❌ DATABASE_CREATE_EVIDENCE: Failed to store media file {}: {}", index + 1, e);
                    println!("   Media ID: {}", media.id);
                    println!("   Filename: {}", media.filename);
                    println!("   Error details: {:?}", e);
                }
            }
        }
        
        // Commit transaction
        conn.execute("COMMIT", [])?;
        
        println!("📊 DATABASE_CREATE_EVIDENCE: Evidence creation summary:");
        println!("   Evidence ID: {}", evidence.id);
        println!("   Evidence Number: {}", evidence.evidence_number);
        println!("   Media files: {} successful, {} failed", media_success, media_failed);
        println!("   Total size: {} bytes", evidence.file_size_bytes);
        
        // Verify the media files were stored
        if media_success > 0 {
            let verify_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM evidence_media WHERE evidence_id = ?",
                [&evidence.id],
                |row| row.get(0)
            )?;
            
            println!("🔍 DATABASE_CREATE_EVIDENCE: Verification - {} media files in database for evidence {}", 
                    verify_count, evidence.id);
            
            if verify_count != media_success as i64 {
                println!("⚠️ DATABASE_CREATE_EVIDENCE: WARNING - Mismatch! Expected {} media files, found {} in database", 
                        media_success, verify_count);
            }
        }
        
        // Log audit
        self.log_audit(
            Some(&evidence.uploader_id),
            "evidence_created",
            "evidence",
            Some(&evidence.id),
            &format!("Evidence created: {} - {} ({} media files)", 
                    evidence.evidence_number, evidence.title, media_success),
            None,
        ).await?;
        
        if media_failed > 0 {
            println!("⚠️ DATABASE_CREATE_EVIDENCE: {} media files failed to store", media_failed);
            return Err(anyhow::anyhow!("Failed to store {} media files", media_failed));
        }
        
        Ok(())
    }
        
    
    
                
    
    pub async fn update_evidence(&self, evidence: &Evidence) -> Result<()> {
        let conn = self.pool.get()?;
        
        println!("💾 DATABASE_UPDATE_EVIDENCE: Updating evidence: {}", evidence.id);
        println!("   Evidence Number: {}", evidence.evidence_number);
        println!("   Title: {}", evidence.title);
        println!("   Media files count: {}", evidence.media_files.len());
        
        // Start transaction
        conn.execute("BEGIN TRANSACTION", [])?;
        
        // Serialize JSON fields
        let mime_types_json = serde_json::to_string(&evidence.mime_types)
            .context("Failed to serialize mime_types")?;
        let hash_values_json = serde_json::to_string(&evidence.hash_values)
            .context("Failed to serialize hash_values")?;
        let storj_urls_json = serde_json::to_string(&evidence.storj_urls)
            .context("Failed to serialize storj_urls")?;
        let chain_of_custody_json = serde_json::to_string(&evidence.chain_of_custody)
            .context("Failed to serialize chain_of_custody")?;
        
        // Update evidence record
        let rows_affected = conn.execute(
            r#"
            UPDATE evidence SET
                emergency_level = ?,
                incident_type = ?,
                sub_type = ?,
                incident_time = ?,
                report_time = ?,
                county = ?,
                constituency = ?,
                ward = ?,
                latitude = ?,
                longitude = ?,
                landmark = ?,
                vehicle_registration = ?,
                vehicle_color = ?,
                vehicle_type = ?,
                sacco_name = ?,
                vehicle_description = ?,
                title = ?,
                description = ?,
                injuries = ?,
                property_damage = ?,
                suspect_description = ?,
                uploader_phone = ?,
                media_count = ?,
                evidence_quality = ?,
                file_size_bytes = ?,
                mime_types = ?,
                hash_values = ?,
                reported_to_police = ?,
                police_case_id = ?,
                police_station = ?,
                report_date = ?,
                wallet_signature = ?,
                wallet_address = ?,
                signature_timestamp = ?,
                status = ?,
                needs_attention = ?,
                is_anonymous = ?,
                chain_of_custody = ?,
                storj_urls = ?,
                storj_bucket = ?,
                updated_at = ?,
                reviewed_at = ?
            WHERE id = ?
            "#,
            params![
                match evidence.emergency_level {
                    EmergencyLevel::Red => "red",
                    EmergencyLevel::Orange => "orange",
                    EmergencyLevel::Yellow => "yellow",
                    EmergencyLevel::Blue => "blue",
                },
                match evidence.incident_type {
                    IncidentType::HitAndRun => "HitAndRun",
                    IncidentType::Assault => "Assault",
                    IncidentType::ThreatToLife => "ThreatToLife",
                    IncidentType::PropertyDamage => "PropertyDamage",
                    IncidentType::Theft => "Theft",
                    IncidentType::Other => "Other",
                },
                evidence.sub_type,
                evidence.incident_time.timestamp(),
                evidence.report_time.timestamp(),
                evidence.location.county,
                evidence.location.constituency,
                evidence.location.ward,
                evidence.location.latitude,
                evidence.location.longitude,
                evidence.location.landmark,
                evidence.vehicle_details.as_ref().and_then(|v| v.registration.clone()),
                evidence.vehicle_details.as_ref().and_then(|v| v.color.clone()),
                evidence.vehicle_details.as_ref().and_then(|v| v.vehicle_type.as_ref().map(|vt| match vt {
                    VehicleType::Matatu => "Matatu",
                    VehicleType::BodaBoda => "BodaBoda",
                    VehicleType::Private => "Private",
                    VehicleType::PSV => "PSV",
                    VehicleType::Lorry => "Lorry",
                    VehicleType::Taxi => "Taxi",
                    VehicleType::Unknown => "Unknown",
                })),
                evidence.vehicle_details.as_ref().and_then(|v| v.sacco_name.clone()),
                evidence.vehicle_details.as_ref().and_then(|v| v.description.clone()),
                evidence.title,
                evidence.description,
                evidence.injuries,
                evidence.property_damage,
                evidence.suspect_description,
                evidence.uploader_phone,
                evidence.media_files.len() as i32,
                evidence.evidence_quality,
                evidence.file_size_bytes as i64,
                mime_types_json,
                hash_values_json,
                evidence.reported_to_police,
                evidence.police_case_id,
                evidence.police_station,
                evidence.report_date.map(|d| d.timestamp()),
                evidence.wallet_signature,
                evidence.wallet_address,
                evidence.signature_timestamp.map(|d| d.timestamp()),
                match evidence.status {
                    EvidenceStatus::Draft => "draft",
                    EvidenceStatus::Submitted => "submitted",
                    EvidenceStatus::Reported => "reported",
                    EvidenceStatus::UnderReview => "under_review",
                    EvidenceStatus::Archived => "archived",
                    EvidenceStatus::Rejected => "rejected",
                },
                evidence.needs_attention,
                evidence.is_anonymous,
                chain_of_custody_json,
                storj_urls_json,
                evidence.storj_bucket,
                evidence.updated_at.timestamp(),
                evidence.reviewed_at.map(|d| d.timestamp()),
                evidence.id,
            ],
        )?;
        
        if rows_affected == 0 {
            conn.execute("ROLLBACK", [])?;
            return Err(anyhow::anyhow!("Evidence not found: {}", evidence.id));
        }
        
        println!("✅ DATABASE_UPDATE_EVIDENCE: Evidence record updated ({} rows affected)", rows_affected);
        
        // Update media files
        println!("📁 DATABASE_UPDATE_EVIDENCE: Processing media files...");
        
        // Delete existing media files
        let deleted_rows = conn.execute(
            "DELETE FROM evidence_media WHERE evidence_id = ?",
            [&evidence.id],
        )?;
        
        println!("🗑️  DATABASE_UPDATE_EVIDENCE: Deleted {} old media files", deleted_rows);
        
        // Insert new media files
        let mut media_success = 0;
        let mut media_failed = 0;
        
        for (index, media) in evidence.media_files.iter().enumerate() {
            println!("📁 DATABASE_UPDATE_EVIDENCE: Storing media file {}: {}", index + 1, media.filename);
            
            // Validate required fields (CRITICAL for live recordings)
            if media.storj_url.is_empty() {
                println!("⚠️ DATABASE_UPDATE_EVIDENCE: Media file {} has empty storj_url", media.filename);
                media_failed += 1;
                continue;
            }
            
            if media.storj_key.is_empty() {
                println!("⚠️ DATABASE_UPDATE_EVIDENCE: Media file {} has empty storj_key", media.filename);
                media_failed += 1;
                continue;
            }
            
            if media.hash.is_empty() {
                println!("⚠️ DATABASE_UPDATE_EVIDENCE: Media file {} has empty hash", media.filename);
                media_failed += 1;
                continue;
            }
            
            match conn.execute(
                r#"
                INSERT INTO evidence_media (
                    id, evidence_id, filename, mime_type, file_size,
                    duration_seconds, thumbnail_url, storj_url, storj_key,
                    hash, quality_rating, description, created_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    media.id,
                    evidence.id,
                    media.filename,
                    media.mime_type,
                    media.file_size as i64,
                    media.duration_seconds,
                    media.thumbnail_url,
                    media.storj_url,
                    media.storj_key,
                    media.hash,
                    media.quality_rating,
                    media.description,
                    evidence.updated_at.timestamp(), // Use updated_at for media creation timestamp
                ],
            ) {
                Ok(_) => {
                    media_success += 1;
                    println!("✅ DATABASE_UPDATE_EVIDENCE: Media file stored: {}", media.filename);
                    println!("   Type: {}", media.mime_type);
                    println!("   Size: {} bytes", media.file_size);
                    
                    // Check if this is a live recording (look for webm, mp4, etc.)
                    let is_live_recording = media.filename.contains("live_recording") || 
                                        media.filename.contains("recording_") ||
                                        media.mime_type.contains("video/");
                    
                    if is_live_recording {
                        println!("🎥 DATABASE_UPDATE_EVIDENCE: This appears to be a live recording");
                        println!("   Storj URL: {}", media.storj_url);
                    }
                }
                Err(e) => {
                    media_failed += 1;
                    println!("❌ DATABASE_UPDATE_EVIDENCE: Failed to store media file {}: {}", index + 1, e);
                }
            }
        }
        
        // Commit transaction
        conn.execute("COMMIT", [])?;
        
        println!("📊 DATABASE_UPDATE_EVIDENCE: Update summary:");
        println!("   Evidence: {}", evidence.evidence_number);
        println!("   Media files: {} successful, {} failed", media_success, media_failed);
        println!("   Total media count in evidence: {}", evidence.media_files.len());
        
        // Verify the media files were stored
        if media_success > 0 {
            let verify_count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM evidence_media WHERE evidence_id = ?",
                [&evidence.id],
                |row| row.get(0)
            )?;
            
            println!("🔍 DATABASE_UPDATE_EVIDENCE: Verification - {} media files in database", verify_count);
            
            if verify_count != media_success as i64 {
                println!("⚠️ DATABASE_UPDATE_EVIDENCE: WARNING - Mismatch! Expected {}, found {}", 
                        media_success, verify_count);
            }
        }
        
        // Log audit
        self.log_audit(
            Some(&evidence.uploader_id),
            "evidence_updated",
            "evidence",
            Some(&evidence.id),
            &format!("Evidence updated: {} - {} ({} media files)", 
                    evidence.evidence_number, evidence.title, media_success),
            None,
        ).await?;
        
        if media_failed > 0 {
            println!("⚠️ DATABASE_UPDATE_EVIDENCE: {} media files failed to store", media_failed);
            // Don't return error here since evidence was updated successfully
            // Just log the warning
        }
        
        Ok(())
    }



    pub async fn get_user_evidence(&self, user_id: &str) -> Result<Vec<EvidenceSummary>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                id, evidence_number, emergency_level, incident_type, title,
                county, incident_time, status, reported_to_police,
                police_case_id, needs_attention
            FROM evidence 
            WHERE uploader_id = ?
            ORDER BY incident_time DESC
            "#,
        )?;
        
        let rows = stmt.query_map([user_id], |row| {
            let emergency_level_str: String = row.get(2)?;
            let emergency_level = match emergency_level_str.as_str() {
                "red" => EmergencyLevel::Red,
                "orange" => EmergencyLevel::Orange,
                "yellow" => EmergencyLevel::Yellow,
                "blue" => EmergencyLevel::Blue,
                _ => EmergencyLevel::Blue,
            };
            
            let incident_type_str: String = row.get(3)?;
            let incident_type = match incident_type_str.as_str() {
                "HitAndRun" => IncidentType::HitAndRun,
                "Assault" => IncidentType::Assault,
                "ThreatToLife" => IncidentType::ThreatToLife,
                "PropertyDamage" => IncidentType::PropertyDamage,
                "Theft" => IncidentType::Theft,
                "Other" => IncidentType::Other,
                _ => IncidentType::Other,
            };
            
            let status_str: String = row.get(7)?;
            let status = match status_str.as_str() {
                "draft" => EvidenceStatus::Draft,
                "submitted" => EvidenceStatus::Submitted,
                "reported" => EvidenceStatus::Reported,
                "under_review" => EvidenceStatus::UnderReview,
                "archived" => EvidenceStatus::Archived,
                "rejected" => EvidenceStatus::Rejected,
                _ => EvidenceStatus::Submitted,
            };
            
            let incident_time: i64 = row.get(6)?;
            
            Ok(EvidenceSummary {
                id: row.get(0)?,
                evidence_number: row.get(1)?,
                emergency_level,
                incident_type,
                title: row.get(4)?,
                county: row.get(5)?,
                incident_time: DateTime::from_timestamp(incident_time, 0).unwrap_or_else(Utc::now),
                status,
                reported_to_police: row.get::<_, i64>(8)? != 0,
                police_case_id: row.get(9)?,
                has_media: true,
                needs_attention: row.get::<_, i64>(10)? != 0,
            })
        })?;
        
        let mut evidence = Vec::new();
        for row in rows {
            evidence.push(row?);
        }
        
        Ok(evidence)
    }
    
    pub async fn get_evidence_stats(&self, user_id: &str) -> Result<DashboardStats> {
        let conn = self.pool.get()?;
        
        // Total evidence
        let total_evidence: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND status != 'rejected'",
            [user_id],
            |row| row.get(0),
        )?;
        
        // Urgent evidence (red + orange)
        let urgent_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND emergency_level IN ('red', 'orange')",
            [user_id],
            |row| row.get(0),
        )?;
        
        // Reported evidence
        let reported_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND reported_to_police = 1",
            [user_id],
            |row| row.get(0),
        )?;
        
        // Needs attention
        let needs_attention_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND needs_attention = 1",
            [user_id],
            |row| row.get(0),
        )?;
        
        // Today's evidence
        let today_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let today_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND incident_time >= ?",
            [user_id, &today_start.to_string()],
            |row| row.get(0),
        )?;
        
        // County stats
        let mut by_county = Vec::new();
        let mut county_stmt = conn.prepare(
            r#"
            SELECT county, COUNT(*) as count,
                   SUM(CASE WHEN emergency_level IN ('red', 'orange') THEN 1 ELSE 0 END) as urgent
            FROM evidence 
            WHERE uploader_id = ?
            GROUP BY county
            ORDER BY count DESC
            LIMIT 10
            "#
        )?;
        
        let county_rows = county_stmt.query_map([user_id], |row| {
            Ok(CountyStats {
                county: row.get(0)?,
                count: row.get(1)?,
                urgent: row.get(2)?,
            })
        })?;
        
        for row in county_rows {
            by_county.push(row?);
        }
        
        // Incident type stats
        let mut by_type = Vec::new();
        let mut type_stmt = conn.prepare(
            r#"
            SELECT incident_type, COUNT(*) as count
            FROM evidence 
            WHERE uploader_id = ?
            GROUP BY incident_type
            ORDER BY count DESC
            "#
        )?;
        
        let type_rows = type_stmt.query_map([user_id], |row| {
            let incident_type_str: String = row.get(0)?;
            let incident_type = match incident_type_str.as_str() {
                "HitAndRun" => IncidentType::HitAndRun,
                "Assault" => IncidentType::Assault,
                "ThreatToLife" => IncidentType::ThreatToLife,
                "PropertyDamage" => IncidentType::PropertyDamage,
                "Theft" => IncidentType::Theft,
                "Other" => IncidentType::Other,
                _ => IncidentType::Other,
            };
            
            Ok(IncidentTypeStats {
                incident_type,
                count: row.get(1)?,
            })
        })?;
        
        for row in type_rows {
            by_type.push(row?);
        }
        
        Ok(DashboardStats {
            total_evidence: total_evidence as u64,
            urgent_count: urgent_count as u64,
            reported_count: reported_count as u64,
            needs_attention_count: needs_attention_count as u64,
            today_count: today_count as u64,
            by_county,
            by_type,
        })
    }
    
    pub async fn get_evidence(&self, evidence_id: &str, increment_views: bool) -> Result<Option<Evidence>> {
        let conn = self.pool.get()?;
        
        println!("💾 DATABASE_GET_EVIDENCE: Loading evidence ID: {}", evidence_id);
        
        // Get evidence record with column names for safety
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                id, evidence_number, emergency_level, incident_type, sub_type,
                incident_time, report_time, county, constituency, ward,
                latitude, longitude, landmark, vehicle_registration, vehicle_color,
                vehicle_type, sacco_name, vehicle_description, title, description,
                injuries, property_damage, suspect_description, uploader_id,
                uploader_email, uploader_phone, media_count, evidence_quality,
                file_size_bytes, mime_types, hash_values, reported_to_police,
                police_case_id, police_station, report_date, wallet_signature,
                wallet_address, signature_timestamp, status, needs_attention,
                is_anonymous, chain_of_custody, storj_urls, storj_bucket,
                created_at, updated_at, reviewed_at
            FROM evidence 
            WHERE id = ?
            "#,
        )?;
        
        let evidence = stmt.query_row([evidence_id], |row| {
            println!("📊 DATABASE_GET_EVIDENCE: Parsing evidence row...");
            
            // Parse JSON fields using column names
            let mime_types_json: String = row.get("mime_types")?;
            let hash_values_json: String = row.get("hash_values")?;
            let storj_urls_json: String = row.get("storj_urls")?;
            let chain_of_custody_json: String = row.get("chain_of_custody")?;
            
            let mime_types: Vec<String> = serde_json::from_str(&mime_types_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            let hash_values: Vec<String> = serde_json::from_str(&hash_values_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            let storj_urls: Vec<String> = serde_json::from_str(&storj_urls_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            let chain_of_custody: Vec<CustodyRecord> = serde_json::from_str(&chain_of_custody_json)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
            
            // Parse enums using column names
            let emergency_level_str: String = row.get("emergency_level")?;
            let emergency_level = match emergency_level_str.as_str() {
                "red" => EmergencyLevel::Red,
                "orange" => EmergencyLevel::Orange,
                "yellow" => EmergencyLevel::Yellow,
                "blue" => EmergencyLevel::Blue,
                _ => EmergencyLevel::Blue,
            };
            
            let incident_type_str: String = row.get("incident_type")?;
            let incident_type = match incident_type_str.as_str() {
                "HitAndRun" => IncidentType::HitAndRun,
                "Assault" => IncidentType::Assault,
                "ThreatToLife" => IncidentType::ThreatToLife,
                "PropertyDamage" => IncidentType::PropertyDamage,
                "Theft" => IncidentType::Theft,
                "Other" => IncidentType::Other,
                _ => IncidentType::Other,
            };
            
            let vehicle_type_str: Option<String> = row.get("vehicle_type")?;
            let vehicle_type = vehicle_type_str.as_ref().map(|vt| match vt.as_str() {
                "Matatu" => VehicleType::Matatu,
                "BodaBoda" => VehicleType::BodaBoda,
                "Private" => VehicleType::Private,
                "PSV" => VehicleType::PSV,
                "Lorry" => VehicleType::Lorry,
                "Taxi" => VehicleType::Taxi,
                _ => VehicleType::Unknown,
            });
            
            // Get values using column names
            let wallet_signature: Option<String> = row.get("wallet_signature")?;
            let wallet_address: Option<String> = row.get("wallet_address")?;
            let signature_timestamp: Option<i64> = row.get("signature_timestamp")?;
            
            let status_str: String = row.get("status")?;
            let status = match status_str.as_str() {
                "draft" => EvidenceStatus::Draft,
                "submitted" => EvidenceStatus::Submitted,
                "reported" => EvidenceStatus::Reported,
                "under_review" => EvidenceStatus::UnderReview,
                "archived" => EvidenceStatus::Archived,
                "rejected" => EvidenceStatus::Rejected,
                _ => EvidenceStatus::Submitted,
            };
            
            // CRITICAL FIX: Load media files directly in the query context
            println!("📁 DATABASE_GET_EVIDENCE: Loading media files for evidence: {}", evidence_id);
            
            // Use the connection from the outer closure
            let mut media_stmt = conn.prepare(
                r#"
                SELECT id, filename, mime_type, file_size, duration_seconds, 
                    thumbnail_url, storj_url, storj_key, hash, quality_rating, description
                FROM evidence_media 
                WHERE evidence_id = ?
                ORDER BY created_at
                "#
            )?;
            
            let media_rows = media_stmt.query_map([evidence_id], |row| {
                Ok(MediaFile {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    mime_type: row.get(2)?,
                    file_size: row.get(3)?,
                    duration_seconds: row.get(4)?,
                    thumbnail_url: row.get(5)?,
                    storj_url: row.get(6)?,
                    storj_key: row.get(7)?,
                    hash: row.get(8)?,
                    quality_rating: row.get(9)?,
                    description: row.get(10)?,
                })
            })?;
            
            let mut media_files = Vec::new();
            let mut success_count = 0;
            let mut error_count = 0;
            
            for row_result in media_rows {
                match row_result {
                    Ok(media) => {
                        success_count += 1;
                        println!("📁 DATABASE_GET_EVIDENCE: Loaded media file: {}", media.filename);
                        media_files.push(media);
                    }
                    Err(e) => {
                        error_count += 1;
                        println!("❌ Error loading media row: {}", e);
                    }
                }
            }
            
            println!("📁 DATABASE_GET_EVIDENCE: Loaded {} media files ({} errors)", success_count, error_count);
            
            // Timestamps using column names
            let incident_time: i64 = row.get("incident_time")?;
            let report_time: i64 = row.get("report_time")?;
            let created_at: i64 = row.get("created_at")?;
            let updated_at: i64 = row.get("updated_at")?;
            
            // Nullable timestamps
            let reviewed_at: Option<i64> = row.get("reviewed_at")?;
            let report_date: Option<i64> = row.get("report_date")?;
            
            // Other fields
            let needs_attention: bool = row.get::<_, i64>("needs_attention")? != 0;
            let is_anonymous: bool = row.get::<_, i64>("is_anonymous")? != 0;
            let reported_to_police: bool = row.get::<_, i64>("reported_to_police")? != 0;
            let media_count: i32 = row.get("media_count")?;
            
            println!("📊 DATABASE_GET_EVIDENCE: Evidence parsed successfully:");
            println!("   Title: {}", row.get::<_, String>("title")?);
            println!("   Media count in DB: {}", media_count);
            println!("   Media files loaded: {}", media_files.len());
            println!("   Has video files: {}", media_files.iter().any(|m| m.mime_type.starts_with("video/")));
            
            Ok(Evidence {
                id: row.get("id")?,
                evidence_number: row.get("evidence_number")?,
                emergency_level,
                incident_type,
                sub_type: row.get("sub_type")?,
                incident_time: DateTime::from_timestamp(incident_time, 0).unwrap_or_else(|| Utc::now()),
                report_time: DateTime::from_timestamp(report_time, 0).unwrap_or_else(|| Utc::now()),
                location: EvidenceLocation {
                    county: row.get("county")?,
                    constituency: row.get("constituency")?,
                    ward: row.get("ward")?,
                    latitude: row.get("latitude")?,
                    longitude: row.get("longitude")?,
                    landmark: row.get("landmark")?,
                    address: None,
                },
                vehicle_details: {
                    let reg: Option<String> = row.get("vehicle_registration")?;
                    if reg.is_some() {
                        Some(VehicleDetails {
                            registration: reg,
                            color: row.get("vehicle_color")?,
                            vehicle_type,
                            make_model: None,
                            sacco_name: row.get("sacco_name")?,
                            description: row.get("vehicle_description")?,
                        })
                    } else {
                        None
                    }
                },
                title: row.get("title")?,
                description: row.get("description")?,
                injuries: row.get("injuries")?,
                property_damage: row.get("property_damage")?,
                suspect_description: row.get("suspect_description")?,
                uploader_id: row.get("uploader_id")?,
                uploader_email: row.get("uploader_email")?,
                uploader_phone: row.get("uploader_phone")?,
                media_files,
                evidence_quality: row.get("evidence_quality")?,
                reported_to_police,
                police_case_id: row.get("police_case_id")?,
                police_station: row.get("police_station")?,
                report_date: report_date.map(|d| DateTime::from_timestamp(d, 0).unwrap_or_else(|| Utc::now())),
                wallet_signature,
                wallet_address,
                signature_timestamp: signature_timestamp.map(|d| DateTime::from_timestamp(d, 0).unwrap_or_else(|| Utc::now())),
                status,
                needs_attention,
                is_anonymous,
                chain_of_custody,
                storj_urls,
                storj_bucket: row.get("storj_bucket")?,
                file_size_bytes: row.get("file_size_bytes")?,
                mime_types,
                hash_values,
                created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(|| Utc::now()),
                updated_at: DateTime::from_timestamp(updated_at, 0).unwrap_or_else(|| Utc::now()),
                reviewed_at: reviewed_at.map(|d| DateTime::from_timestamp(d, 0).unwrap_or_else(|| Utc::now())),
            })
        }).optional()?;
        
        if increment_views && evidence.is_some() {
            self.log_audit(
                None,
                "evidence_viewed",
                "evidence",
                Some(evidence_id),
                "Evidence viewed",
                None,
            ).await?;
        }
        
        Ok(evidence)
    }

    // Helper method to get media files
    fn get_evidence_media(&self, conn: &rusqlite::Connection, evidence_id: &str) -> Result<Vec<MediaFile>> {
        let mut stmt = conn.prepare(
            r#"
            SELECT id, evidence_id, filename, mime_type, file_size,
                duration_seconds, thumbnail_url, storj_url, storj_key,
                hash, quality_rating, description, created_at
            FROM evidence_media 
            WHERE evidence_id = ?
            ORDER BY created_at
            "#
        )?;
        
        let rows = stmt.query_map([evidence_id], |row| {
            Ok(MediaFile {
                id: row.get(0)?,
                filename: row.get(2)?,
                mime_type: row.get(3)?,
                file_size: row.get(4)?,
                duration_seconds: row.get(5)?,
                thumbnail_url: row.get(6)?,
                storj_url: row.get(7)?,
                storj_key: row.get(8)?,
                hash: row.get(9)?,
                quality_rating: row.get(10)?,
                description: row.get(11)?,
            })
        })?;
        
        let mut media_files = Vec::new();
        let mut success_count = 0;
        let mut error_count = 0;
        
        for row_result in rows {
            match row_result {
                Ok(media) => {
                    success_count += 1;
                    media_files.push(media);
                }
                Err(e) => {
                    error_count += 1;
                    println!("❌ Error loading media row: {}", e);
                }
            }
        }
        
        println!("📁 GET_EVIDENCE_MEDIA: Success: {}, Errors: {}", success_count, error_count);
        Ok(media_files)
    }




    pub async fn store_evidence_signature(&self, signature: &EvidenceSignature) -> Result<()> {
        let conn = self.pool.get()?;
        let signature_id = format!("sig_{}", Uuid::new_v4());
        
        conn.execute(
            r#"
            INSERT INTO evidence_signatures (
                id, evidence_id, wallet_address, signature, signed_hash,
                timestamp, chain, transaction_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                signature_id,
                signature.evidence_id,
                signature.wallet_address,
                signature.signature,
                signature.signed_hash,
                signature.timestamp.timestamp(),
                signature.chain,
                signature.transaction_id
            ],
        )?;
        
        self.log_audit(
            Some(&signature.wallet_address),
            "evidence_signed",
            "evidence",
            Some(&signature.evidence_id),
            &format!("Evidence signed by {}", signature.wallet_address),
            None,
        ).await?;
        
        Ok(())
    }

    
    
    pub async fn get_evidence_signatures(&self, evidence_id: &str) -> Result<Vec<EvidenceSignature>> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                evidence_id, wallet_address, signature, signed_hash,
                timestamp, chain, transaction_id
            FROM evidence_signatures 
            WHERE evidence_id = ?
            ORDER BY timestamp DESC
            "#,
        )?;
        
        let rows = stmt.query_map([evidence_id], |row| {
            let timestamp: i64 = row.get(4)?;
            
            Ok(EvidenceSignature {
                evidence_id: row.get(0)?,
                wallet_address: row.get(1)?,
                signature: row.get(2)?,
                signed_hash: row.get(3)?,
                timestamp: DateTime::from_timestamp(timestamp, 0)
                    .unwrap_or_else(Utc::now),
                chain: row.get(5)?,
                transaction_id: row.get(6)?,
            })
        })?;
        
        let mut signatures = Vec::new();
        for row in rows {
            signatures.push(row?);
        }
        
        Ok(signatures)
    }

  
    // In database.rs,  get_all_evidence

    pub async fn get_all_evidence(&self) -> Result<Vec<Evidence>> {
        let conn = self.pool.get()?;
        
        // First get all evidence records
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                id, evidence_number, emergency_level, incident_type, sub_type,
                incident_time, report_time, county, constituency, ward,
                latitude, longitude, landmark, vehicle_registration, vehicle_color,
                vehicle_type, sacco_name, vehicle_description, title, description,
                injuries, property_damage, suspect_description, uploader_id,
                uploader_email, uploader_phone, media_count, evidence_quality,
                file_size_bytes, mime_types, hash_values, reported_to_police,
                police_case_id, police_station, report_date, wallet_signature,
                wallet_address, signature_timestamp, status, needs_attention,
                is_anonymous, chain_of_custody, storj_urls, storj_bucket,
                created_at, updated_at, reviewed_at
            FROM evidence 
            ORDER BY incident_time DESC
            "#,
        )?;
        
        let evidence_rows = stmt.query_map([], |row| {
            // Get basic evidence data
            let id: String = row.get(0)?;
            
            Ok(id)
        })?;
        
        let mut evidence_ids = Vec::new();
        for row in evidence_rows {
            evidence_ids.push(row?);
        }
        
        println!("📊 DATABASE: Found {} evidence records", evidence_ids.len());
        
        // Now load each evidence with its media files
        let mut all_evidence = Vec::new();
        
        for evidence_id in evidence_ids {
            match self.get_evidence(&evidence_id, false).await {
                Ok(Some(evidence)) => {
                    all_evidence.push(evidence);
                }
                Ok(None) => {
                    println!("⚠️ Evidence {} not found when loading all evidence", evidence_id);
                }
                Err(e) => {
                    println!("⚠️ Error loading evidence {}: {}", evidence_id, e);
                }
            }
        }
        
        println!("📊 DATABASE: Successfully loaded {} evidence records", all_evidence.len());
        Ok(all_evidence)
    }


    // Add these methods to the Database impl block:

    pub async fn create_target(&self, target: &TargetPhoto) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            r#"
            INSERT INTO targets (
                id, evidence_id, target_number, filename, mime_type,
                file_size, description, category, confidence_score,
                storj_url, storj_key, hash, phash, auto_generated,
                created_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                target.id,
                target.evidence_id,
                target.target_number,
                target.filename,
                target.mime_type,
                target.file_size as i64,
                target.description,
                target.category.as_str(),
                target.confidence_score,
                target.storj_url,
                target.storj_key,
                target.hash,
                target.phash,
                target.auto_generated as i32,
                target.created_at.timestamp(),
                target.created_by,
            ],
        )?;
        
        Ok(())
    }

    pub async fn get_targets_for_evidence(&self, evidence_id: &str) -> Result<Vec<TargetPhoto>> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                id, evidence_id, target_number, filename, mime_type,
                file_size, description, category, confidence_score,
                storj_url, storj_key, hash, phash, auto_generated,
                created_at, created_by
            FROM targets 
            WHERE evidence_id = ?
            ORDER BY target_number
            "#,
        )?;
        
        let rows = stmt.query_map([evidence_id], |row| {
            let category_str: String = row.get(7)?;
            let category = match category_str.as_str() {
                "person" => TargetCategory::Person,
                "vehicle" => TargetCategory::Vehicle,
                "object" => TargetCategory::Object,
                "location" => TargetCategory::Location,
                "other" => TargetCategory::Other,
                _ => TargetCategory::Other,
            };
            
            Ok(TargetPhoto {
                id: row.get(0)?,
                evidence_id: row.get(1)?,
                target_number: row.get(2)?,
                filename: row.get(3)?,
                mime_type: row.get(4)?,
                file_size: row.get(5)?,
                description: row.get(6)?,
                category,
                confidence_score: row.get(8)?,
                storj_url: row.get(9)?,
                storj_key: row.get(10)?,
                hash: row.get(11)?,
                phash: row.get(12)?,
                auto_generated: row.get::<_, i32>(13)? != 0,
                created_at: DateTime::from_timestamp(row.get(14)?, 0)
                    .unwrap_or_else(|| Utc::now()),
                created_by: row.get(15)?,
            })
        })?;
        
        let mut targets = Vec::new();
        for row in rows {
            targets.push(row?);
        }
        
        Ok(targets)
    }

    pub async fn delete_targets_for_evidence(&self, evidence_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            "DELETE FROM targets WHERE evidence_id = ?",
            [evidence_id],
        )?;
        
        Ok(())
    }

    // Add to src/database.rs, inside the Database implementation

    // In database.rs, inside the Database implementation
    pub async fn cleanup_old_records(&self) -> Result<CleanupStats> {
        println!("🧹 DATABASE: Cleaning up old records");
        
        let conn = self.pool.get()?;
        
        // Calculate timestamps for cleanup
        let now = Utc::now().timestamp();
        let one_year_ago = now - (365 * 24 * 60 * 60);
        let six_months_ago = now - (6 * 30 * 24 * 60 * 60);
        let one_week_ago = now - (7 * 24 * 60 * 60);
        let thirty_days_ago = now - (30 * 24 * 60 * 60);
        
        // Delete users who haven't logged in for more than 1 year and aren't verified
        let deleted_users = conn.execute(
            "DELETE FROM users WHERE is_verified = 0 AND last_login_at < ?1",
            params![one_year_ago],
        )?;
        
        // Delete old audit logs (older than 6 months)
        let deleted_audit_logs = conn.execute(
            "DELETE FROM audit_log WHERE created_at < ?1",
            params![six_months_ago],
        )?;
        
        // Delete old verification tokens (older than 1 week)
        let deleted_temp_files = conn.execute(
            "DELETE FROM verification_tokens WHERE created_at < ?1",
            params![one_week_ago],
        )?;
        
        // Delete draft evidence that's older than 30 days and hasn't been updated
        let deleted_old_content = conn.execute(
            "DELETE FROM evidence WHERE status = 'draft' AND updated_at < ?1",
            params![thirty_days_ago],
        )?;
        
        println!("✅ DATABASE: Cleanup completed");
        println!("   Deleted users: {}", deleted_users);
        println!("   Deleted old content: {}", deleted_old_content);
        println!("   Deleted audit logs: {}", deleted_audit_logs);
        println!("   Deleted temp files: {}", deleted_temp_files);
        
        Ok(CleanupStats {
            deleted_users: deleted_users as i64,
            deleted_old_content: deleted_old_content as i64,
            deleted_audit_logs: deleted_audit_logs as i64,
            deleted_temp_files: deleted_temp_files as i64,
        })
    }

        // Add to src/database.rs, inside the Database implementation
    
    // Then update the method signature:
    pub async fn get_all_evidence_locations(&self) -> Result<Vec<EvidenceLocationData>> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                id, evidence_number, title, emergency_level, incident_type, 
                county, latitude, longitude, incident_time, status
            FROM evidence 
            WHERE status != 'rejected' 
            AND latitude IS NOT NULL 
            AND longitude IS NOT NULL
            AND latitude != 0 
            AND longitude != 0
            ORDER BY incident_time DESC
            "#
        )?;
        
        let rows = stmt.query_map([], |row| {
            let emergency_level_str: String = row.get(3)?;
            let emergency_level = match emergency_level_str.as_str() {
                "red" => EmergencyLevel::Red,
                "orange" => EmergencyLevel::Orange,
                "yellow" => EmergencyLevel::Yellow,
                "blue" => EmergencyLevel::Blue,
                _ => EmergencyLevel::Blue,
            };
            
            let incident_type_str: String = row.get(4)?;
            let incident_type = match incident_type_str.as_str() {
                "HitAndRun" => IncidentType::HitAndRun,
                "Assault" => IncidentType::Assault,
                "ThreatToLife" => IncidentType::ThreatToLife,
                "PropertyDamage" => IncidentType::PropertyDamage,
                "Theft" => IncidentType::Theft,
                "Other" => IncidentType::Other,
                _ => IncidentType::Other,
            };
            
            let status_str: String = row.get(9)?;
            let status = match status_str.as_str() {
                "draft" => EvidenceStatus::Draft,
                "submitted" => EvidenceStatus::Submitted,
                "reported" => EvidenceStatus::Reported,
                "under_review" => EvidenceStatus::UnderReview,
                "archived" => EvidenceStatus::Archived,
                "rejected" => EvidenceStatus::Rejected,
                _ => EvidenceStatus::Submitted,
            };
            
            let incident_time: i64 = row.get(8)?;
            
            Ok(EvidenceLocationData {
                id: row.get(0)?,
                evidence_number: row.get(1)?,
                title: row.get(2)?,
                emergency_level,
                incident_type,
                county: row.get(5)?,
                latitude: row.get(6)?,
                longitude: row.get(7)?,
                incident_time: DateTime::from_timestamp(incident_time, 0).unwrap_or_else(Utc::now),
                status,
            })
        })?;
        
        let mut locations = Vec::new();
        for row in rows {
            locations.push(row?);
        }
        
        Ok(locations)
    }


    // Add to src/database.rs, inside the Database implementation
    pub async fn get_targets_statistics(&self) -> Result<(i64, i64)> {
        let conn = self.pool.get()?;
        
        // Get total number of targets
        let total_targets: i64 = conn.query_row(
            "SELECT COUNT(*) FROM targets",
            [],
            |row| row.get(0)
        )?;
        
        // Get total size of all targets in bytes
        let total_size_bytes: i64 = conn.query_row(
            "SELECT COALESCE(SUM(file_size), 0) FROM targets",
            [],
            |row| row.get(0)
        )?;
        
        Ok((total_targets, total_size_bytes))
    }

    pub async fn get_evidence_statistics(&self) -> Result<(i64, i64)> {
        let conn = self.pool.get()?;
        
        // Get total number of evidence
        let total_evidence: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE status != 'rejected'",
            [],
            |row| row.get(0)
        )?;
        
        // Get total size of all evidence in bytes
        let total_size_bytes: i64 = conn.query_row(
            "SELECT COALESCE(SUM(file_size_bytes), 0) FROM evidence WHERE status != 'rejected'",
            [],
            |row| row.get(0)
        )?;
        
        Ok((total_evidence, total_size_bytes))
    }

    pub async fn get_audit_statistics(&self) -> Result<i64> {
        let conn = self.pool.get()?;
        
        let total_audit: i64 = conn.query_row(
            "SELECT COUNT(*) FROM audit_log",
            [],
            |row| row.get(0)
        )?;
        
        Ok(total_audit)
    }

    pub async fn get_all_statistics(&self) -> Result<StatisticsResponse> {
        let (total_targets, targets_size_bytes) = self.get_targets_statistics().await?;
        let (total_evidence, evidence_size_bytes) = self.get_evidence_statistics().await?;
        let total_audit = self.get_audit_statistics().await?;
        
        Ok(StatisticsResponse {
            total_targets,
            targets_size_bytes,
            total_evidence,
            evidence_size_bytes,
            total_audit,
        })
    }


    // Add to database.rs inside the Database implementation
pub async fn get_evidence_locations_with_filters(
    &self,
    filters: &EvidenceLocationFilters,
) -> Result<Vec<EvidenceLocationDataSup>> {
    let conn = self.pool.get()?;
    
    let mut where_clauses = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    
    // Build filter conditions
    if let Some(county) = &filters.county {
        if county != "all" && !county.is_empty() {
            where_clauses.push("county = ?".to_string());
            params.push(Box::new(county.to_string()));
        }
    }
    
    if let Some(emergency_level) = &filters.emergency_level {
        if emergency_level != "all" && !emergency_level.is_empty() {
            where_clauses.push("emergency_level = ?".to_string());
            params.push(Box::new(emergency_level.to_string()));
        }
    }
    
    if let Some(incident_type) = &filters.incident_type {
        if incident_type != "all" && !incident_type.is_empty() {
            where_clauses.push("incident_type = ?".to_string());
            params.push(Box::new(incident_type.to_string()));
        }
    }
    
    if let Some(status) = &filters.status {
        if status != "all" && !status.is_empty() {
            where_clauses.push("status = ?".to_string());
            params.push(Box::new(status.to_string()));
        }
    }
    
    if let Some(reported) = filters.reported_to_police {
        where_clauses.push("reported_to_police = ?".to_string());
        params.push(Box::new(if reported { 1 } else { 0 }));
    }
    
    // Date range filter
    if let Some(date_from) = &filters.date_from {
        if !date_from.is_empty() {
            where_clauses.push("incident_time >= ?".to_string());
            params.push(Box::new(date_from.to_string()));
        }
    }
    
    if let Some(date_to) = &filters.date_to {
        if !date_to.is_empty() {
            where_clauses.push("incident_time <= ?".to_string());
            params.push(Box::new(date_to.to_string()));
        }
    }
    
    // Always include basic filters
    where_clauses.push("status != 'rejected'".to_string());
    where_clauses.push("latitude IS NOT NULL".to_string());
    where_clauses.push("longitude IS NOT NULL".to_string());
    where_clauses.push("latitude != 0".to_string());
    where_clauses.push("longitude != 0".to_string());
    
    let where_clause = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };
    
    let query = format!(
    r#"
    SELECT 
        id, evidence_number, title, emergency_level, incident_type, 
        county, latitude, longitude, incident_time, status,
        reported_to_police, police_case_id, uploader_email,
        created_at, media_count, needs_attention
    FROM evidence 
    {}
    ORDER BY incident_time DESC
    "#,
    where_clause
);
    
    let mut stmt = conn.prepare(&query)?;
    
    let rows = stmt.query_map(
    params.iter().map(|p| &**p).collect::<Vec<_>>().as_slice(),
    |row| {
        let emergency_level_str: String = row.get(3)?;
        let emergency_level = match emergency_level_str.as_str() {
            "red" => EmergencyLevel::Red,
            "orange" => EmergencyLevel::Orange,
            "yellow" => EmergencyLevel::Yellow,
            "blue" => EmergencyLevel::Blue,
            _ => EmergencyLevel::Blue,
        };
        
        let incident_type_str: String = row.get(4)?;
        let incident_type = match incident_type_str.as_str() {
            "HitAndRun" => IncidentType::HitAndRun,
            "Assault" => IncidentType::Assault,
            "ThreatToLife" => IncidentType::ThreatToLife,
            "PropertyDamage" => IncidentType::PropertyDamage,
            "Theft" => IncidentType::Theft,
            "Other" => IncidentType::Other,
            _ => IncidentType::Other,
        };
        
        let status_str: String = row.get(9)?;
        let status = match status_str.as_str() {
            "draft" => EvidenceStatus::Draft,
            "submitted" => EvidenceStatus::Submitted,
            "reported" => EvidenceStatus::Reported,
            "under_review" => EvidenceStatus::UnderReview,
            "archived" => EvidenceStatus::Archived,
            "rejected" => EvidenceStatus::Rejected,
            _ => EvidenceStatus::Submitted,
        };
        
        let incident_time: i64 = row.get(8)?;
        let created_at: i64 = row.get(13)?;
        
        Ok(EvidenceLocationDataSup {
            id: row.get(0)?,
            evidence_number: row.get(1)?,
            title: row.get(2)?,
            emergency_level,
            incident_type,
            county: row.get(5)?,
            latitude: row.get(6)?,
            longitude: row.get(7)?,
            incident_time: DateTime::from_timestamp(incident_time, 0).unwrap_or_else(Utc::now),
            status,
            // Add these fields that were missing:
            reported_to_police: row.get::<_, i64>(10)? != 0,
            police_case_id: row.get(11)?,
            uploader_email: row.get(12)?,
            created_at: DateTime::from_timestamp(created_at, 0).unwrap_or_else(Utc::now),
            media_count: row.get(14)?,
            needs_attention: row.get::<_, i64>(15)? != 0,
        })
    },
    )?;
    
    let mut locations = Vec::new();
    for row in rows {
        locations.push(row?);
    }
    
    Ok(locations)
}

pub async fn get_evidence_map_statistics(&self, filters: &EvidenceLocationFilters) -> Result<EvidenceMapStatistics> {
    let conn = self.pool.get()?;
    
    // Build filter conditions
    let mut where_clauses = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();
    
    if let Some(county) = &filters.county {
        if county != "all" && !county.is_empty() {
            where_clauses.push("county = ?".to_string());
            params.push(Box::new(county.to_string()));
        }
    }
    
    if let Some(emergency_level) = &filters.emergency_level {
        if emergency_level != "all" && !emergency_level.is_empty() {
            where_clauses.push("emergency_level = ?".to_string());
            params.push(Box::new(emergency_level.to_string()));
        }
    }
    
    // Always include basic filters
    where_clauses.push("status != 'rejected'".to_string());
    where_clauses.push("latitude IS NOT NULL".to_string());
    where_clauses.push("longitude IS NOT NULL".to_string());
    
    let where_clause = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };
    
    // Total evidence
    let total_evidence: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM evidence {}", where_clause),
        params.iter().map(|p| &**p).collect::<Vec<_>>().as_slice(),
        |row| row.get(0),
    )?;
    
    // Urgent evidence (red + orange)
    let urgent_count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM evidence {} AND emergency_level IN ('red', 'orange')", 
                where_clause),
        params.iter().map(|p| &**p).collect::<Vec<_>>().as_slice(),
        |row| row.get(0),
    )?;
    
    // Reported evidence
    let reported_count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM evidence {} AND reported_to_police = 1", 
                where_clause),
        params.iter().map(|p| &**p).collect::<Vec<_>>().as_slice(),
        |row| row.get(0),
    )?;
    
    // County distribution
    let mut county_stats = HashMap::new();
    let mut stmt = conn.prepare(
        &format!(
            r#"
            SELECT county, COUNT(*) as count,
                   SUM(CASE WHEN emergency_level IN ('red', 'orange') THEN 1 ELSE 0 END) as urgent
            FROM evidence 
            {}
            GROUP BY county
            ORDER BY count DESC
            LIMIT 10
            "#,
            where_clause
        )
    )?;
    
    let rows = stmt.query_map(
        params.iter().map(|p| &**p).collect::<Vec<_>>().as_slice(),
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        },
    )?;
    
    for row in rows {
        let (county, count, urgent) = row?;
        county_stats.insert(county, (count, urgent));
    }
    
    // Incident type distribution
    let mut incident_stats = HashMap::new();
    let mut type_stmt = conn.prepare(
        &format!(
            r#"
            SELECT incident_type, COUNT(*) as count
            FROM evidence 
            {}
            GROUP BY incident_type
            "#,
            where_clause
        )
    )?;
    
    let type_rows = type_stmt.query_map(
        params.iter().map(|p| &**p).collect::<Vec<_>>().as_slice(),
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
            ))
        },
    )?;
    
    for row in type_rows {
        let (incident_type, count) = row?;
        incident_stats.insert(incident_type, count);
    }
    
    Ok(EvidenceMapStatistics {
        total_evidence,
        urgent_count,
        reported_count,
        county_stats,
        incident_stats,
    })
}


// Add to database.rs inside the Database implementation
pub async fn get_evidence_chart_data(&self, user_id: &str) -> Result<ChartData> {
    let conn = self.pool.get()?;
    
    // Get counts for each category
    // 1. Collated (total evidence)
    let collated: i64 = conn.query_row(
        "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND status != 'rejected'",
        [user_id],
        |row| row.get(0),
    )?;
    
    // 2. Reported (reported to police)
    let reported: i64 = conn.query_row(
        "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND reported_to_police = 1",
        [user_id],
        |row| row.get(0),
    )?;
    
    // 3. Submitted (status = 'submitted')
    let submitted: i64 = conn.query_row(
        "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND status = 'submitted'",
        [user_id],
        |row| row.get(0),
    )?;
    
    // 4. Draft (status = 'draft')
    let draft: i64 = conn.query_row(
        "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND status = 'draft'",
        [user_id],
        |row| row.get(0),
    )?;
    
    // 5. Urgent (red or orange emergency level)
    let urgent: i64 = conn.query_row(
        "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND emergency_level IN ('red', 'orange')",
        [user_id],
        |row| row.get(0),
    )?;
    
    // 6. Signed (has wallet signature)
    let signed: i64 = conn.query_row(
        "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND wallet_signature IS NOT NULL AND wallet_signature != ''",
        [user_id],
        |row| row.get(0),
    )?;
    
    // 7. Others (any other status or category)
    let others: i64 = conn.query_row(
        "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND status NOT IN ('draft', 'submitted', 'reported')",
        [user_id],
        |row| row.get(0),
    )?;
    
    Ok(ChartData {
        collated,
        reported,
        submitted,
        draft,
        urgent,
        signed,
        others,
    })
}


 pub async fn get_storage_statistics(&self, user_id: &str) -> Result<MediaStorageStats> {
        let conn = self.pool.get()?;
        
        // 1. Media - count of media files for user's evidence
        let media_count: i64 = conn.query_row(
            r#"
            SELECT COUNT(*) 
            FROM evidence_media 
            WHERE evidence_id IN (
                SELECT id FROM evidence WHERE uploader_id = ?
            )
            "#,
            [user_id],
            |row| row.get(0),
        )?;
        
        // 2. Scenes - count of distinct counties where evidence was recorded
        let scenes_count: i64 = conn.query_row(
            r#"
            SELECT COUNT(DISTINCT county) 
            FROM evidence 
            WHERE uploader_id = ? AND county IS NOT NULL AND county != ''
            "#,
            [user_id],
            |row| row.get(0),
        )?;
        
        // 3. Profiles - count of targets created (could be persons/vehicles)
        let profiles_count: i64 = conn.query_row(
            r#"
            SELECT COUNT(*) 
            FROM targets 
            WHERE evidence_id IN (
                SELECT id FROM evidence WHERE uploader_id = ?
            )
            "#,
            [user_id],
            |row| row.get(0),
        )?;
        
        // 4. Evidence - total evidence count
        let evidence_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND status != 'rejected'",
            [user_id],
            |row| row.get(0),
        )?;
        
        // 5. Target - count of targets with 'person' category
        let target_count: i64 = conn.query_row(
            r#"
            SELECT COUNT(*) 
            FROM targets 
            WHERE category = 'person' 
            AND evidence_id IN (
                SELECT id FROM evidence WHERE uploader_id = ?
            )
            "#,
            [user_id],
            |row| row.get(0),
        )?;
        
        Ok(MediaStorageStats {
            media: media_count,
            scenes: scenes_count,
            profiles: profiles_count,
            evidence: evidence_count,
            target: target_count,
        })
    }
    
    pub async fn get_storage_size_statistics(&self, user_id: &str) -> Result<StorageSizeData> {
        let conn = self.pool.get()?;
        
        // Media storage size in MB
        let media_size_mb: i64 = conn.query_row(
            r#"
            SELECT COALESCE(SUM(file_size) / 1024 / 1024, 0)
            FROM evidence_media 
            WHERE evidence_id IN (
                SELECT id FROM evidence WHERE uploader_id = ?
            )
            "#,
            [user_id],
            |row| row.get(0),
        )?;
        
        // Evidence storage size in MB
        let evidence_size_mb: i64 = conn.query_row(
            r#"
            SELECT COALESCE(SUM(file_size_bytes) / 1024 / 1024, 0)
            FROM evidence 
            WHERE uploader_id = ?
            "#,
            [user_id],
            |row| row.get(0),
        )?;
        
        // Targets storage size in MB
        let target_size_mb: i64 = conn.query_row(
            r#"
            SELECT COALESCE(SUM(file_size) / 1024 / 1024, 0)
            FROM targets 
            WHERE evidence_id IN (
                SELECT id FROM evidence WHERE uploader_id = ?
            )
            "#,
            [user_id],
            |row| row.get(0),
        )?;
        
        Ok(StorageSizeData {
            media: media_size_mb,
            evidence: evidence_size_mb,
            target: target_size_mb,
        })
    }

    // ==================== TARGET HASH MATCHING & EVIDENCE LINKING ====================

    /// Check if a target hash already exists and return all evidence IDs that have it
    pub async fn check_target_hash_exists(&self, hash: &str) -> Result<Vec<(String, String, String, i32)>> {
        let conn = self.pool.get()?;
        
        let mut stmt = conn.prepare(
            r#"
            SELECT t.evidence_id, t.filename, t.category, t.confidence_score
            FROM targets t
            WHERE t.hash = ?
            ORDER BY t.created_at DESC
            "#,
        )?;
        
        let results = stmt.query_map([hash], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i32>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(results)
    }

    /// Search stored pHashes for visually similar images using Hamming distance.
    ///
    /// `query_phash`  — 16-char hex pHash from the JS sidecar
    /// `max_distance` — maximum Hamming distance to consider a match (recommended: 10)
    ///
    /// Returns all matching targets sorted by distance ascending (closest first).
    /// Only searches targets where phash IS NOT NULL and auto_generated = 0
    /// (i.e. user-confirmed targets only, not auto-created face splits).
    pub async fn search_phash_matches(
        &self,
        query_phash:  &str,
        max_distance: u32,            // recommended: 10
        exclude_evidence_id: &str,    // don't match against the same case
    ) -> Result<Vec<PHashMatchResult>> {
        let conn = self.pool.get()?;

        // Pull all stored pHashes in one pass — Hamming distance computed in Rust
        let mut stmt = conn.prepare(
            r#"
            SELECT
                t.id, t.evidence_id, t.filename, t.category,
                t.description, t.confidence_score, t.phash, t.created_at,
                e.evidence_number, e.uploader_id, e.uploader_email
            FROM targets t
            JOIN evidence e ON t.evidence_id = e.id
            WHERE t.phash IS NOT NULL
              AND t.auto_generated = 0
              AND t.evidence_id != ?1
            "#,
        )?;

        // Parse query hash once
        let query_bits = match u64::from_str_radix(query_phash, 16) {
            Ok(v) => v,
            Err(_) => {
                println!("⚠️  Invalid pHash format: {}", query_phash);
                return Ok(vec![]);
            }
        };

        let rows = stmt.query_map(params![exclude_evidence_id], |row| {
            Ok((
                row.get::<_, String>(0)?,   // target id
                row.get::<_, String>(1)?,   // evidence_id
                row.get::<_, String>(2)?,   // filename
                row.get::<_, String>(3)?,   // category
                row.get::<_, Option<String>>(4)?,  // description
                row.get::<_, i32>(5)?,      // confidence_score
                row.get::<_, String>(6)?,   // phash
                row.get::<_, i64>(7)?,      // created_at
                row.get::<_, String>(8)?,   // evidence_number
                row.get::<_, String>(9)?,   // uploader_id
                row.get::<_, String>(10)?,  // uploader_email
            ))
        })?;

        let mut matches: Vec<PHashMatchResult> = Vec::new();

        for row in rows {
            let (
                target_id, evidence_id, filename, category,
                description, confidence_score, stored_phash, created_at,
                evidence_number, uploader_id, uploader_email,
            ) = row?;

            // Parse stored hash — skip malformed entries
            let stored_bits = match u64::from_str_radix(&stored_phash, 16) {
                Ok(v) => v,
                Err(_) => {
                    println!("⚠️  Skipping malformed pHash for target {}", target_id);
                    continue;
                }
            };

            // Hamming distance = popcount of XOR
            let hamming = (query_bits ^ stored_bits).count_ones();

            if hamming <= max_distance {
                // Convert distance to 0–100 confidence
                // distance=0 → 100%, distance=max_distance → 0%
                let confidence = ((1.0 - hamming as f64 / max_distance as f64) * 100.0)
                    .round() as i32;

                matches.push(PHashMatchResult {
                    target_id,
                    evidence_id,
                    filename,
                    category,
                    description,
                    confidence_score,
                    stored_phash,
                    hamming_distance: hamming,
                    confidence_pct: confidence,
                    created_at,
                    evidence_number,
                    uploader_id,
                    uploader_email,
                });
            }
        }

        // Sort by Hamming distance ascending — best match first
        matches.sort_by_key(|m| m.hamming_distance);

        println!(
            "🔍 pHash search complete: {} match(es) (max_distance={})",
            matches.len(), max_distance
        );

        Ok(matches)
    }

    // ─────────────────────────────────────────────────────────────────────────
    // FACE ENCODING FUNCTIONS
    // ─────────────────────────────────────────────────────────────────────────

    /// Persist a face descriptor (128 × f32) for a target into face_encodings.
    /// `descriptor` must be exactly 128 elements — serialised as 512 LE bytes.
    /// `phash` is the 16-char hex pHash computed by the JS sidecar (fallback).
    /// `face_index` is 0 for the first / largest face in the image.
    /// `auto_generated` = true means the face was split from a multi-face upload,
    /// not explicitly chosen by the uploader as a target.
    pub async fn insert_face_encoding(
        &self,
        target_id:       &str,
        evidence_id:     &str,
        face_index:      i32,
        descriptor:      &[f32],   // must be len 128
        detection_score: f64,
        phash:           Option<&str>,
        auto_generated:  bool,
    ) -> Result<String> {
        if descriptor.len() != 128 {
            return Err(anyhow!(
                "face descriptor must be 128 floats, got {}",
                descriptor.len()
            ));
        }

        // Serialise 128 × f32 → 512 bytes (little-endian)
        let mut blob = Vec::with_capacity(512);
        for &v in descriptor {
            blob.extend_from_slice(&v.to_le_bytes());
        }

        let id   = format!("fenc_{}", Uuid::new_v4());
        let now  = Utc::now().timestamp();
        let conn = self.pool.get()?;

        conn.execute(
            r#"
            INSERT INTO face_encodings
                (id, target_id, evidence_id, face_index, descriptor,
                 detection_score, phash, auto_generated, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            params![
                id,
                target_id,
                evidence_id,
                face_index,
                blob,
                detection_score,
                phash,
                auto_generated as i32,
                now,
            ],
        )?;

        println!(
            "🧬 Stored face encoding {} for target {} (face_index={}, score={:.3}, auto={})",
            &id[..12], &target_id[..12.min(target_id.len())],
            face_index, detection_score, auto_generated
        );

        Ok(id)
    }

    /// Compare `query_descriptor` against every stored face encoding and return
    /// all records whose Euclidean distance is ≤ `threshold` (recommended: 0.55).
    ///
    /// Returns a Vec of `FaceMatchResult` sorted by distance ascending (closest
    /// match first).  The Euclidean distance is computed in Rust because SQLite
    /// has no vector math — this is fine for thousands of encodings; revisit with
    /// an ANN index if the table grows beyond ~100 k rows.
    pub async fn search_face_encodings(
        &self,
        query_descriptor: &[f32],  // must be len 128
        threshold:        f64,     // e.g. 0.55
    ) -> Result<Vec<FaceMatchResult>> {
        if query_descriptor.len() != 128 {
            return Err(anyhow!(
                "query descriptor must be 128 floats, got {}",
                query_descriptor.len()
            ));
        }

        let conn = self.pool.get()?;

        // Pull every stored encoding with its metadata in one pass
        let mut stmt = conn.prepare(
            r#"
            SELECT
                fe.id, fe.target_id, fe.evidence_id, fe.face_index,
                fe.descriptor, fe.detection_score, fe.phash, fe.auto_generated,
                fe.created_at,
                t.category, t.description,
                e.evidence_number, e.uploader_id, e.uploader_email
            FROM face_encodings fe
            JOIN targets  t ON fe.target_id   = t.id
            JOIN evidence e ON fe.evidence_id = e.id
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,   // encoding id
                row.get::<_, String>(1)?,   // target_id
                row.get::<_, String>(2)?,   // evidence_id
                row.get::<_, i32>(3)?,      // face_index
                row.get::<_, Vec<u8>>(4)?,  // descriptor blob
                row.get::<_, f64>(5)?,      // detection_score
                row.get::<_, Option<String>>(6)?,  // phash
                row.get::<_, i32>(7)?,      // auto_generated
                row.get::<_, i64>(8)?,      // created_at
                row.get::<_, String>(9)?,   // category
                row.get::<_, String>(10)?,  // description
                row.get::<_, String>(11)?,  // evidence_number
                row.get::<_, String>(12)?,  // uploader_id
                row.get::<_, String>(13)?,  // uploader_email
            ))
        })?;

        let mut matches: Vec<FaceMatchResult> = Vec::new();

        for row in rows {
            let (
                encoding_id, target_id, evidence_id, face_index,
                blob, detection_score, phash, auto_generated, created_at,
                category, description, evidence_number, uploader_id, uploader_email,
            ) = row?;

            // Deserialise blob → [f32; 128]
            if blob.len() != 512 {
                println!("⚠️  Skipping malformed encoding {} (blob len={})", encoding_id, blob.len());
                continue;
            }

            let stored: Vec<f32> = blob
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();

            // Euclidean distance in f64 for precision
            let distance = query_descriptor
                .iter()
                .zip(stored.iter())
                .map(|(&a, &b)| {
                    let diff = a as f64 - b as f64;
                    diff * diff
                })
                .sum::<f64>()
                .sqrt();

            if distance <= threshold {
                // Convert distance to a 0–100 confidence score
                // distance=0.0 → 100%, distance=threshold → 0%
                let confidence = ((1.0 - distance / threshold) * 100.0).round() as i32;

                matches.push(FaceMatchResult {
                    encoding_id,
                    target_id,
                    evidence_id,
                    face_index,
                    distance,
                    confidence_score: confidence,
                    detection_score,
                    phash,
                    auto_generated: auto_generated != 0,
                    created_at,
                    category,
                    description,
                    evidence_number,
                    uploader_id,
                    uploader_email,
                });
            }
        }

        // Sort by distance ascending — best match first
        matches.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));

        println!(
            "🔍 Face search complete: {} match(es) found (threshold={:.2})",
            matches.len(), threshold
        );

        Ok(matches)
    }

    /// Return all face encodings stored for a given evidence_id.
    /// Useful for cleanup and for re-processing.
    pub async fn get_face_encodings_for_evidence(
        &self,
        evidence_id: &str,
    ) -> Result<Vec<FaceEncodingRecord>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, target_id, evidence_id, face_index,
                   descriptor, detection_score, phash, auto_generated, created_at
            FROM   face_encodings
            WHERE  evidence_id = ?1
            ORDER  BY created_at ASC
            "#,
        )?;

        let records = stmt.query_map(params![evidence_id], |row| {
            let blob: Vec<u8> = row.get(4)?;
            let descriptor: Vec<f32> = blob
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();

            Ok(FaceEncodingRecord {
                id:              row.get(0)?,
                target_id:       row.get(1)?,
                evidence_id:     row.get(2)?,
                face_index:      row.get(3)?,
                descriptor,
                detection_score: row.get(5)?,
                phash:           row.get(6)?,
                auto_generated:  row.get::<_, i32>(7)? != 0,
                created_at:      row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(records)
    }

    /// Look up all face encodings for a single target_id.
    /// A target can have more than one encoding when multiple faces were
    /// found in the same image.
    pub async fn get_face_encodings_for_target(
        &self,
        target_id: &str,
    ) -> Result<Vec<FaceEncodingRecord>> {
        let conn = self.pool.get()?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, target_id, evidence_id, face_index,
                   descriptor, detection_score, phash, auto_generated, created_at
            FROM   face_encodings
            WHERE  target_id = ?1
            ORDER  BY face_index ASC
            "#,
        )?;

        let records = stmt.query_map(params![target_id], |row| {
            let blob: Vec<u8> = row.get(4)?;
            let descriptor: Vec<f32> = blob
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();

            Ok(FaceEncodingRecord {
                id:              row.get(0)?,
                target_id:       row.get(1)?,
                evidence_id:     row.get(2)?,
                face_index:      row.get(3)?,
                descriptor,
                detection_score: row.get(5)?,
                phash:           row.get(6)?,
                auto_generated:  row.get::<_, i32>(7)? != 0,
                created_at:      row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

        Ok(records)
    }

    // ─────────────────────────────────────────────────────────────────────────

    /// Link two evidence cases together
    pub async fn link_evidence_cases(
        &self,
        evidence_id_1: &str,
        evidence_id_2: &str,
        link_type: &str,
        link_reason: &str,
        matched_target_hash: &str,
        confidence_score: i32,
        created_by: Option<&str>,
    ) -> Result<String> {
        let conn = self.pool.get()?;
        let link_id = format!("link_{}", Uuid::new_v4());
        
        // Ensure evidence_id_1 < evidence_id_2 alphabetically to prevent duplicates
        let (eid1, eid2) = if evidence_id_1 < evidence_id_2 {
            (evidence_id_1, evidence_id_2)
        } else {
            (evidence_id_2, evidence_id_1)
        };
        
        conn.execute(
            r#"
            INSERT OR IGNORE INTO linked_evidence (
                id, evidence_id_1, evidence_id_2, link_type, link_reason,
                matched_target_hash, confidence_score, created_at, created_by
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                link_id,
                eid1,
                eid2,
                link_type,
                link_reason,
                matched_target_hash,
                confidence_score,
                Utc::now().timestamp(),
                created_by,
            ],
        )?;
        
        println!("🔗 Linked evidence {} <-> {} (type: {}, hash: {}...)", 
            eid1, eid2, link_type, &matched_target_hash[..8]);
        
        Ok(link_id)
    }

    /// Get all evidence linked to a specific evidence ID
    pub async fn get_linked_evidence(&self, evidence_id: &str) -> Result<Vec<LinkedEvidenceRecord>> {
        let conn = self.pool.get()?;
        
        // UNION both tables:
        //   linked_evidence  — legacy table (created_at stored as INTEGER unix secs)
        //   linked_cases     — new API table (created_at stored as TEXT RFC3339)
        // Both are cast to TEXT here so the UNION column type is consistent.
        let mut stmt = conn.prepare(
            r#"
            SELECT 
                le.id, le.evidence_id_1, le.evidence_id_2, le.link_type, 
                le.link_reason, le.matched_target_hash, le.confidence_score,
                CAST(le.created_at AS TEXT) AS created_at, le.created_by,
                e1.evidence_number, e1.title, e1.emergency_level, e1.uploader_email,
                e2.evidence_number, e2.title, e2.emergency_level, e2.uploader_email
            FROM linked_evidence le
            LEFT JOIN evidence e1 ON le.evidence_id_1 = e1.id
            LEFT JOIN evidence e2 ON le.evidence_id_2 = e2.id
            WHERE le.evidence_id_1 = ?1 OR le.evidence_id_2 = ?1

            UNION

            SELECT 
                lc.id, lc.evidence_id_1, lc.evidence_id_2, lc.link_type,
                lc.link_reason, lc.matched_target_hash, lc.confidence_score,
                lc.created_at, lc.created_by,
                e1.evidence_number, e1.title, e1.emergency_level, e1.uploader_email,
                e2.evidence_number, e2.title, e2.emergency_level, e2.uploader_email
            FROM linked_cases lc
            LEFT JOIN evidence e1 ON lc.evidence_id_1 = e1.id
            LEFT JOIN evidence e2 ON lc.evidence_id_2 = e2.id
            WHERE lc.evidence_id_1 = ?1 OR lc.evidence_id_2 = ?1

            ORDER BY created_at DESC
            "#,
        )?;
        
        let records = stmt.query_map(params![evidence_id], |row| {
            let evidence_id_1: String = row.get(1)?;
            let evidence_id_2: String = row.get(2)?;
            
            // Determine which is the "other" evidence
            let is_first = evidence_id == evidence_id_1;
            let other_evidence_id = if is_first { evidence_id_2.clone() } else { evidence_id_1.clone() };
            let other_evidence_number: String = row.get(if is_first { 13 } else { 9 })?;
            let other_title: String = row.get(if is_first { 14 } else { 10 })?;
            let other_emergency_level: String = row.get(if is_first { 15 } else { 11 })?;
            let other_uploader_email: String = row.get(if is_first { 16 } else { 12 })?;
            
            Ok(LinkedEvidenceRecord {
                link_id: row.get(0)?,
                evidence_id_1,
                evidence_id_2,
                link_type: row.get(3)?,
                link_reason: row.get(4)?,
                matched_target_hash: row.get(5)?,
                confidence_score: row.get(6)?,
                created_at: {
                    // linked_evidence stores unix secs (cast to TEXT), 
                    // linked_cases stores RFC3339 strings.
                    let s: String = row.get(7)?;
                    if let Ok(secs) = s.parse::<i64>() {
                        DateTime::from_timestamp(secs, 0).unwrap_or_else(|| Utc::now())
                    } else {
                        DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now())
                    }
                },
                created_by: row.get(8)?,
                other_evidence_id,
                other_evidence_number,
                other_title,
                other_emergency_level,
                other_uploader_email,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(records)
    }

    /// Create a notification for a user
    pub async fn create_notification(
        &self,
        user_id: &str,
        notification_type: &str,
        title: &str,
        message: &str,
        evidence_id: Option<&str>,
        linked_evidence_id: Option<&str>,
        target_hash: Option<&str>,
    ) -> Result<String> {
        let conn = self.pool.get()?;
        let notification_id = format!("notif_{}", Uuid::new_v4());
        
        conn.execute(
            r#"
            INSERT INTO notifications (
                id, user_id, notification_type, title, message,
                evidence_id, linked_evidence_id, target_hash,
                is_read, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?)
            "#,
            params![
                notification_id,
                user_id,
                notification_type,
                title,
                message,
                evidence_id,
                linked_evidence_id,
                target_hash,
                Utc::now().timestamp(),
            ],
        )?;
        
        println!("🔔 Created notification for user {}: {}", user_id, title);
        
        Ok(notification_id)
    }

    /// Get all notifications for a user
    pub async fn get_user_notifications(
        &self,
        user_id: &str,
        include_read: bool,
    ) -> Result<Vec<NotificationRecord>> {
        let conn = self.pool.get()?;
        
        let query = if include_read {
            r#"
            SELECT id, user_id, notification_type, title, message,
                   evidence_id, linked_evidence_id, target_hash,
                   is_read, created_at, read_at
            FROM notifications
            WHERE user_id = ?
            ORDER BY created_at DESC
            LIMIT 50
            "#
        } else {
            r#"
            SELECT id, user_id, notification_type, title, message,
                   evidence_id, linked_evidence_id, target_hash,
                   is_read, created_at, read_at
            FROM notifications
            WHERE user_id = ? AND is_read = 0
            ORDER BY created_at DESC
            LIMIT 50
            "#
        };
        
        let mut stmt = conn.prepare(query)?;
        
        let notifications = stmt.query_map([user_id], |row| {
            Ok(NotificationRecord {
                id: row.get(0)?,
                user_id: row.get(1)?,
                notification_type: row.get(2)?,
                title: row.get(3)?,
                message: row.get(4)?,
                evidence_id: row.get(5)?,
                linked_evidence_id: row.get(6)?,
                target_hash: row.get(7)?,
                is_read: row.get(8)?,
                created_at: DateTime::from_timestamp(row.get(9)?, 0)
                    .unwrap_or_else(|| Utc::now()),
                read_at: row.get::<_, Option<i64>>(10)?
                    .and_then(|ts| DateTime::from_timestamp(ts, 0)),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
        
        Ok(notifications)
    }

    /// Mark a notification as read
    pub async fn mark_notification_read(&self, notification_id: &str) -> Result<()> {
        let conn = self.pool.get()?;
        
        conn.execute(
            r#"
            UPDATE notifications
            SET is_read = 1, read_at = ?
            WHERE id = ?
            "#,
            params![Utc::now().timestamp(), notification_id],
        )?;
        
        Ok(())
    }

    /// Get unread notification count for a user
    pub async fn get_unread_notification_count(&self, user_id: &str) -> Result<i64> {
        let conn = self.pool.get()?;
        
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM notifications WHERE user_id = ? AND is_read = 0",
            [user_id],
            |row| row.get(0),
        )?;
        
        Ok(count)
    }

    // ═══════════════════════════════════════════════════════════════
    // SETTINGS — generic JSON key-value store
    // ═══════════════════════════════════════════════════════════════

    /// Load a typed settings struct. Returns None if not yet saved (caller uses Default).
    pub async fn get_setting_json<T>(&self, user_id: &str, key: &str) -> Option<T>
    where
        T: for<'de> serde::de::Deserialize<'de>,
    {
        let conn = self.pool.get().ok()?;
        let result: rusqlite::Result<String> = conn.query_row(
            "SELECT value_json FROM platform_settings WHERE user_id = ?1 AND key = ?2",
            params![user_id, key],
            |row| row.get(0),
        );
        let json_str = result.ok()?;
        serde_json::from_str(&json_str).ok()
    }

    /// Persist a typed settings struct, upserting by (user_id, key).
    pub async fn set_setting_json<T>(&self, user_id: &str, key: &str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        let conn     = self.pool.get()?;
        let json_str = serde_json::to_string(value)
            .context("Failed to serialize settings")?;
        let id  = format!("{}:{}", user_id, key);
        let now = Utc::now().timestamp();

        conn.execute(
            r#"
            INSERT INTO platform_settings (id, user_id, key, value_json, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ON CONFLICT(user_id, key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at
            "#,
            params![id, user_id, key, json_str, now],
        )?;
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════
    // PERSONS OF INTEREST
    // ═══════════════════════════════════════════════════════════════

    pub async fn get_poi_list(&self, user_id: &str) -> Result<Vec<crate::settings_routes::PoiProfile>> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT id, poi_number, display_name, category, status,
                   linked_cases, linked_evidence, notes, pinned_by,
                   created_at, last_seen_at, resolved_at
            FROM persons_of_interest
            WHERE pinned_by = ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let rows = stmt.query_map(params![user_id], |row| {
            let evidence_json: String = row.get(6)?;
            Ok(crate::settings_routes::PoiProfile {
                id:              row.get(0)?,
                poi_number:      row.get(1)?,
                display_name:    row.get(2)?,
                category:        row.get(3)?,
                status:          row.get(4)?,
                linked_cases:    row.get(5)?,
                linked_evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
                notes:           row.get(7)?,
                pinned_by:       row.get(8)?,
                created_at:      row.get(9)?,
                last_seen_at:    row.get(10)?,
                resolved_at:     row.get(11)?,
            })
        })?;

        let mut pois = Vec::new();
        for row in rows { pois.push(row?); }
        Ok(pois)
    }

    pub async fn get_poi(
        &self,
        poi_id:  &str,
        user_id: &str,
    ) -> Result<Option<crate::settings_routes::PoiProfile>> {
        let conn = self.pool.get()?;
        let result = conn.query_row(
            r#"
            SELECT id, poi_number, display_name, category, status,
                   linked_cases, linked_evidence, notes, pinned_by,
                   created_at, last_seen_at, resolved_at
            FROM persons_of_interest
            WHERE id = ?1 AND pinned_by = ?2
            "#,
            params![poi_id, user_id],
            |row| {
                let evidence_json: String = row.get(6)?;
                Ok(crate::settings_routes::PoiProfile {
                    id:              row.get(0)?,
                    poi_number:      row.get(1)?,
                    display_name:    row.get(2)?,
                    category:        row.get(3)?,
                    status:          row.get(4)?,
                    linked_cases:    row.get(5)?,
                    linked_evidence: serde_json::from_str(&evidence_json).unwrap_or_default(),
                    notes:           row.get(7)?,
                    pinned_by:       row.get(8)?,
                    created_at:      row.get(9)?,
                    last_seen_at:    row.get(10)?,
                    resolved_at:     row.get(11)?,
                })
            },
        ).optional()?;
        Ok(result)
    }

    pub async fn create_poi(&self, poi: &crate::settings_routes::PoiProfile) -> Result<()> {
        let conn          = self.pool.get()?;
        let evidence_json = serde_json::to_string(&poi.linked_evidence)
            .context("Failed to serialize linked_evidence")?;

        conn.execute(
            r#"
            INSERT INTO persons_of_interest
                (id, poi_number, display_name, category, status,
                 linked_cases, linked_evidence, notes, pinned_by,
                 created_at, last_seen_at, resolved_at)
            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)
            "#,
            params![
                poi.id,
                poi.poi_number,
                poi.display_name,
                poi.category,
                poi.status,
                poi.linked_cases,
                evidence_json,
                poi.notes,
                poi.pinned_by,
                poi.created_at,
                poi.last_seen_at,
                poi.resolved_at,
            ],
        )?;
        Ok(())
    }

    pub async fn update_poi_status(
        &self,
        poi_id:  &str,
        user_id: &str,
        status:  &str,
    ) -> Result<()> {
        let conn     = self.pool.get()?;
        let now      = Utc::now().timestamp();
        let resolved: Option<i64> = if status == "resolved" { Some(now) } else { None };

        conn.execute(
            r#"
            UPDATE persons_of_interest
            SET status = ?1, resolved_at = ?2, last_seen_at = ?3
            WHERE id = ?4 AND pinned_by = ?5
            "#,
            params![status, resolved, now, poi_id, user_id],
        )?;
        Ok(())
    }

    /// Archives every non-archived POI owned by this user. Returns row count.
    pub async fn archive_all_poi(&self, user_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        let n = conn.execute(
            r#"
            UPDATE persons_of_interest
            SET status = 'archived'
            WHERE pinned_by = ?1 AND status != 'archived'
            "#,
            params![user_id],
        )?;
        Ok(n)
    }

    // ═══════════════════════════════════════════════════════════════
    // DANGER ZONE HELPERS
    // ═══════════════════════════════════════════════════════════════

    /// Purges sessions older than 24 h from the sessions table (if it exists).
    pub async fn purge_expired_sessions(&self) -> Result<usize> {
        let conn   = self.pool.get()?;
        let cutoff = Utc::now().timestamp() - (24 * 3600);
        // actix-session CookieSessionStore has no DB table; this is a no-op guard.
        // If you later switch to a DB-backed session store, the table will be here.
        let n = conn.execute(
            "DELETE FROM sessions WHERE last_activity < ?1",
            params![cutoff],
        ).unwrap_or(0);
        Ok(n)
    }

    /// Hard-deletes all archived evidence rows for a user (cascade removes media/targets).
    pub async fn delete_archived_evidence(&self, user_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        let n = conn.execute(
            "DELETE FROM evidence WHERE status = 'archived' AND uploader_id = ?1",
            params![user_id],
        )?;
        Ok(n)
    }

    /// Wipes the entire notifications table for a user. Returns row count deleted.
    pub async fn wipe_user_notifications(&self, user_id: &str) -> Result<usize> {
        let conn = self.pool.get()?;
        let n = conn.execute(
            "DELETE FROM notifications WHERE user_id = ?1",
            params![user_id],
        )?;
        Ok(n)
    }

    /// ⚡ IRREVERSIBLE — clears all user data except the users row and audit_log.
    pub async fn full_platform_reset(&self, user_id: &str) -> Result<serde_json::Value> {
        let conn = self.pool.get()?;

        conn.execute("BEGIN TRANSACTION", [])?;

        let deleted_evidence: usize = conn.execute(
            "DELETE FROM evidence WHERE uploader_id = ?1",
            params![user_id],
        )?;
        let deleted_poi: usize = conn.execute(
            "DELETE FROM persons_of_interest WHERE pinned_by = ?1",
            params![user_id],
        )?;
        let deleted_notif: usize = conn.execute(
            "DELETE FROM notifications WHERE user_id = ?1",
            params![user_id],
        )?;
        let deleted_settings: usize = conn.execute(
            "DELETE FROM platform_settings WHERE user_id = ?1",
            params![user_id],
        )?;

        conn.execute("COMMIT", [])?;

        self.log_audit(
            Some(user_id),
            "danger_full_reset",
            "platform",
            None,
            &format!(
                "Full reset: {} evidence, {} POI, {} notifications, {} settings deleted",
                deleted_evidence, deleted_poi, deleted_notif, deleted_settings
            ),
            None,
        ).await.ok();

        Ok(serde_json::json!({
            "deleted_evidence":      deleted_evidence,
            "deleted_poi":           deleted_poi,
            "deleted_notifications": deleted_notif,
            "deleted_settings":      deleted_settings,
        }))
    }

    // ═══════════════════════════════════════════════════════════════
// PATCH: Add this method to `impl Database` in src/database.rs
// Insertion point: just before the closing `}` of `impl Database`
// (around line 3562, before the standalone structs at the bottom)
// ═══════════════════════════════════════════════════════════════

    /// Returns all flagged targets (POI / watchlist / pinned / takedown / flagged)
    /// aggregated across every user, with enriched evidence metadata.
    /// Used exclusively by GET /api/watchlist/billboard.
    ///
    /// Returns three data sets in one DB round-trip:
    ///   `records`  — every submitted/reported/under_review evidence record,
    ///                ordered by urgency.  Populates the main billboard grid.
    ///   `targets`  — targets flagged by at least one user (POI/watchlist etc.).
    ///                Populates the targets layer.
    ///   `activity` — last 30 target-related audit events for the live feed.
    pub async fn get_billboard_data(&self) -> Result<serde_json::Value> {
        let conn = self.pool.get()?;

        // ── 1. All publicly visible evidence records ──────────────────────
        let mut ev_stmt = conn.prepare(r#"
            SELECT
                e.id,
                COALESCE(e.evidence_number, '—')                            AS evidence_number,
                COALESCE(e.title, e.description, '')                        AS description,
                COALESCE(e.incident_type, 'other')                          AS incident_type,
                COALESCE(e.county, '—')                                     AS county,
                e.emergency_level,
                e.status,
                e.incident_time,
                (e.media_count > 0)                                         AS has_media,
                e.reported_to_police,
                e.needs_attention,

                -- Scene image: prefer thumbnail, else full storj_url,
                -- from the first (earliest) evidence_media row.
                COALESCE((
                    SELECT COALESCE(NULLIF(em.thumbnail_url,''), em.storj_url)
                    FROM evidence_media em
                    WHERE em.evidence_id = e.id
                    ORDER BY em.created_at ASC LIMIT 1
                ), '') AS image_url,

                -- Wallet-signed flag
                CASE
                    WHEN e.wallet_address  IS NOT NULL AND e.wallet_address  != ''
                     AND e.wallet_signature IS NOT NULL AND e.wallet_signature != ''
                    THEN 1 ELSE 0
                END AS wallet_signed,

                -- Average confidence from linked targets (default 75)
                CAST(COALESCE((
                    SELECT AVG(t.confidence_score)
                    FROM targets t WHERE t.evidence_id = e.id
                ), 75) AS INTEGER) AS confidence_score,

                -- Flag roll-ups across every target of this evidence
                COALESCE((SELECT MAX(COALESCE(tf.is_poi,0))
                    FROM targets t JOIN target_flags tf ON tf.target_id=t.id
                    WHERE t.evidence_id=e.id), 0) AS is_poi,
                COALESCE((SELECT MAX(COALESCE(tf.is_watchlist,0))
                    FROM targets t JOIN target_flags tf ON tf.target_id=t.id
                    WHERE t.evidence_id=e.id), 0) AS is_watchlist,
                COALESCE((SELECT MAX(COALESCE(tf.is_pinned,0))
                    FROM targets t JOIN target_flags tf ON tf.target_id=t.id
                    WHERE t.evidence_id=e.id), 0) AS is_pinned,
                COALESCE((SELECT MAX(COALESCE(tf.is_takedown,0))
                    FROM targets t JOIN target_flags tf ON tf.target_id=t.id
                    WHERE t.evidence_id=e.id), 0) AS is_takedown,
                COALESCE((SELECT MAX(COALESCE(tf.is_flagged,0))
                    FROM targets t JOIN target_flags tf ON tf.target_id=t.id
                    WHERE t.evidence_id=e.id), 0) AS is_flagged,

                -- Total linked cases from BOTH tables
                COALESCE((
                    SELECT COUNT(*) FROM linked_evidence le
                    WHERE le.evidence_id_1=e.id OR le.evidence_id_2=e.id
                ), 0) +
                COALESCE((
                    SELECT COUNT(*) FROM linked_cases lc
                    WHERE lc.evidence_id_1=e.id OR lc.evidence_id_2=e.id
                ), 0) AS linked_cases_count,

                -- Audit activity count
                COALESCE((
                    SELECT COUNT(*) FROM audit_log al WHERE al.action_target=e.id
                ), 0) AS report_count,

                e.created_at

            FROM evidence e
            WHERE e.status NOT IN ('draft', 'archived', 'rejected')
            ORDER BY
                e.needs_attention         DESC,
                CASE e.emergency_level
                    WHEN 'red'    THEN 1
                    WHEN 'orange' THEN 2
                    WHEN 'yellow' THEN 3
                    ELSE               4
                END                       ASC,
                e.created_at              DESC
            LIMIT 1000
        "#)?;

        let records: Vec<serde_json::Value> = ev_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,    // id
                row.get::<_, String>(1)?,    // evidence_number
                row.get::<_, String>(2)?,    // description
                row.get::<_, String>(3)?,    // incident_type
                row.get::<_, String>(4)?,    // county
                row.get::<_, String>(5)?,    // emergency_level
                row.get::<_, String>(6)?,    // status
                row.get::<_, i64>(7)?,       // incident_time
                row.get::<_, bool>(8)?,      // has_media
                row.get::<_, bool>(9)?,      // reported_to_police
                row.get::<_, bool>(10)?,     // needs_attention
                row.get::<_, String>(11)?,   // image_url
                row.get::<_, i64>(12)?,      // wallet_signed
                row.get::<_, i64>(13)?,      // confidence_score
                row.get::<_, i64>(14)?,      // is_poi
                row.get::<_, i64>(15)?,      // is_watchlist
                row.get::<_, i64>(16)?,      // is_pinned
                row.get::<_, i64>(17)?,      // is_takedown
                row.get::<_, i64>(18)?,      // is_flagged
                row.get::<_, i64>(19)?,      // linked_cases_count
                row.get::<_, i64>(20)?,      // report_count
                row.get::<_, i64>(21)?,      // created_at
            ))
        })?
        .filter_map(|r| r.ok())
        .map(|(
            id, evidence_number, description, incident_type,
            county, emergency_level, status, incident_time_secs,
            has_media, reported_to_police, needs_attention,
            image_url, wallet_signed, confidence_score,
            is_poi, is_watchlist, is_pinned, is_takedown, is_flagged,
            linked_cases_count, report_count, created_at_secs,
        )| {
            serde_json::json!({
                "id":               id,
                "evidence_id":      id,
                "evidence_number":  evidence_number,
                "evidenceNumber":   evidence_number,
                "description":      description,
                "title":            description,
                "incident_type":    incident_type,
                "category":         incident_type,
                "county":           county,
                "emergency_level":  emergency_level,
                "emergencyLevel":   emergency_level,
                "status":           status,
                "incident_time":    incident_time_secs * 1000,
                "created_at":       created_at_secs * 1000,
                "createdAt":        created_at_secs * 1000,
                "image_url":        image_url,
                "imageUrl":         image_url,
                "has_media":        has_media,
                "wallet_signed":    wallet_signed != 0,
                "signedByWallet":   wallet_signed != 0,
                "signed_by_wallet": wallet_signed != 0,
                "confidence_score": confidence_score,
                "confidence":       confidence_score,
                "is_poi":           is_poi != 0,
                "poi":              is_poi != 0,
                "is_watchlist":     is_watchlist != 0,
                "watchlist":        is_watchlist != 0,
                "is_pinned":        is_pinned != 0,
                "pinned":           is_pinned != 0,
                "is_takedown":      is_takedown != 0,
                "takedown":         is_takedown != 0,
                "is_flagged":       is_flagged != 0,
                "flagged":          is_flagged != 0,
                "linked_cases_count": linked_cases_count,
                "linkedCases":      linked_cases_count,
                "report_count":     report_count,
                "reportCount":      report_count,
                "reported_to_police": reported_to_police,
                "needs_attention":  needs_attention,
            })
        })
        .collect();

        // ── 2. Flagged targets layer ───────────────────────────────────────
        let mut tgt_stmt = conn.prepare(r#"
            SELECT
                t.id, t.evidence_id,
                COALESCE(e.evidence_number, '—')    AS evidence_number,
                COALESCE(t.category, 'other')       AS category,
                COALESCE(t.description, '')         AS description,
                COALESCE(NULLIF(t.storj_url,''),'') AS image_url,
                COALESCE(t.confidence_score, 50)    AS confidence_score,
                t.created_at,
                COALESCE(e.emergency_level, 'blue') AS emergency_level,
                COALESCE(e.county, '—')             AS county,
                MAX(COALESCE(tf.is_poi,      0))    AS is_poi,
                MAX(COALESCE(tf.is_watchlist,0))    AS is_watchlist,
                MAX(COALESCE(tf.is_pinned,   0))    AS is_pinned,
                MAX(COALESCE(tf.is_takedown, 0))    AS is_takedown,
                MAX(COALESCE(tf.is_flagged,  0))    AS is_flagged,
                COALESCE((SELECT notes FROM target_flags
                    WHERE target_id=t.id ORDER BY updated_at DESC LIMIT 1), '') AS notes,
                COALESCE((SELECT SUM(json_array_length(linked_case_refs))
                    FROM target_flags
                    WHERE target_id=t.id
                      AND linked_case_refs IS NOT NULL AND linked_case_refs!='[]'
                ), 0) AS linked_case_count,
                COALESCE((SELECT COUNT(*) FROM audit_log WHERE target_id=t.id), 0) AS report_count
            FROM targets t
            LEFT JOIN evidence     e  ON e.id          = t.evidence_id
            LEFT JOIN target_flags tf ON tf.target_id  = t.id
            GROUP BY t.id
            HAVING
                MAX(COALESCE(tf.is_poi,      0))=1
             OR MAX(COALESCE(tf.is_watchlist, 0))=1
             OR MAX(COALESCE(tf.is_pinned,    0))=1
             OR MAX(COALESCE(tf.is_takedown,  0))=1
             OR MAX(COALESCE(tf.is_flagged,   0))=1
            ORDER BY
                MAX(COALESCE(tf.is_poi,      0)) DESC,
                MAX(COALESCE(tf.is_takedown, 0)) DESC,
                MAX(COALESCE(tf.is_pinned,   0)) DESC,
                MAX(COALESCE(tf.is_watchlist,0)) DESC,
                t.created_at DESC
            LIMIT 500
        "#)?;

        let targets: Vec<serde_json::Value> = tgt_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,    // id
                row.get::<_, String>(1)?,    // evidence_id
                row.get::<_, String>(2)?,    // evidence_number
                row.get::<_, String>(3)?,    // category
                row.get::<_, String>(4)?,    // description
                row.get::<_, String>(5)?,    // image_url
                row.get::<_, i64>(6)?,       // confidence_score
                row.get::<_, i64>(7)?,       // created_at
                row.get::<_, String>(8)?,    // emergency_level
                row.get::<_, String>(9)?,    // county
                row.get::<_, i64>(10)?,      // is_poi
                row.get::<_, i64>(11)?,      // is_watchlist
                row.get::<_, i64>(12)?,      // is_pinned
                row.get::<_, i64>(13)?,      // is_takedown
                row.get::<_, i64>(14)?,      // is_flagged
                row.get::<_, String>(15)?,   // notes
                row.get::<_, i64>(16)?,      // linked_case_count
                row.get::<_, i64>(17)?,      // report_count
            ))
        })?
        .filter_map(|r| r.ok())
        .map(|(
            id, evidence_id, evidence_number,
            category, description, image_url,
            confidence_score, created_at_secs, emergency_level, county,
            is_poi, is_watchlist, is_pinned, is_takedown, is_flagged,
            notes, linked_case_count, report_count,
        )| {
            serde_json::json!({
                "id":               id,
                "evidence_id":      evidence_id,
                "evidenceId":       evidence_id,
                "evidence_number":  evidence_number,
                "evidenceNumber":   evidence_number,
                "category":         category,
                "incident_type":    category,
                "description":      description,
                "image_url":        image_url,
                "imageUrl":         image_url,
                "confidence_score": confidence_score,
                "confidence":       confidence_score,
                "created_at":       created_at_secs * 1000,
                "createdAt":        created_at_secs * 1000,
                "emergency_level":  emergency_level,
                "emergencyLevel":   emergency_level,
                "county":           county,
                "notes":            notes,
                "is_poi":           is_poi != 0,
                "poi":              is_poi != 0,
                "is_watchlist":     is_watchlist != 0,
                "watchlist":        is_watchlist != 0,
                "is_pinned":        is_pinned != 0,
                "pinned":           is_pinned != 0,
                "is_takedown":      is_takedown != 0,
                "takedown":         is_takedown != 0,
                "is_flagged":       is_flagged != 0,
                "flagged":          is_flagged != 0,
                "linked_cases_count": linked_case_count,
                "linkedCases":      linked_case_count,
                "report_count":     report_count,
                "reportCount":      report_count,
            })
        })
        .collect();

        // ── 3. Live activity feed ─────────────────────────────────────────
        let mut act_stmt = conn.prepare(r#"
            SELECT action_type, details, created_at
            FROM   audit_log
            WHERE  action_type LIKE 'target_%'
            ORDER BY created_at DESC
            LIMIT 30
        "#)?;

        let activity: Vec<serde_json::Value> = act_stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .enumerate()
        .map(|(i, (action_type, details, created_at_secs))| {
            let (event_type, icon) = match action_type.as_str() {
                "target_poi"       => ("poi",       "🎯"),
                "target_watchlist" => ("watchlist", "👁"),
                "target_pin"       => ("pin",       "📌"),
                "target_takedown"  => ("takedown",  "🚫"),
                "target_flag"      => ("complaint", "🚩"),
                "target_notes"     => ("report",    "📝"),
                "target_link_case" => ("link",      "🔗"),
                _                  => ("report",    "📡"),
            };
            let diff = (chrono::Utc::now().timestamp() - created_at_secs).max(0);
            let time_ago = if diff < 60 { format!("{}s ago", diff) }
                else if diff < 3600  { format!("{}m ago", diff/60) }
                else if diff < 86400 { format!("{}h ago", diff/3600) }
                else                 { format!("{}d ago", diff/86400) };
            serde_json::json!({
                "id": i, "type": event_type, "icon": icon,
                "message": details, "timeAgo": time_ago, "isNew": false,
            })
        })
        .collect();

        let records_len = records.len();
        let targets_len = targets.len();
        Ok(serde_json::json!({
            "records":       records,
            "targets":       targets,
            "activity":      activity,
            "total":         records_len,
            "targets_total": targets_len,
        }))
    }
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLogEntry {
    pub id: String,
    pub user_id: Option<String>,
    pub action_type: String,
    pub action_target: String,
    pub target_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DatabaseStats {
    pub total_users: i64,
    pub total_evidence: i64,
    pub urgent_evidence: i64,
    pub reported_evidence: i64,
    pub total_wallet_connections: i64,
    pub total_audit_logs: i64,
    pub database_size_bytes: u64,
}

// Add to src/models.rs or src/database.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupStats {
    pub deleted_users: i64,
    pub deleted_old_content: i64,
    pub deleted_audit_logs: i64,
    pub deleted_temp_files: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// FACE ENCODING STRUCTS
// ─────────────────────────────────────────────────────────────────────────────

/// A face encoding record as stored in the database (descriptor already
/// deserialised back to Vec<f32> for convenience).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceEncodingRecord {
    pub id:              String,
    pub target_id:       String,
    pub evidence_id:     String,
    pub face_index:      i32,
    pub descriptor:      Vec<f32>,   // 128 floats
    pub detection_score: f64,
    pub phash:           Option<String>,
    pub auto_generated:  bool,
    pub created_at:      i64,
}

/// Returned by `search_phash_matches()` — one entry per target whose pHash
/// is within `max_distance` Hamming bits of the query image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PHashMatchResult {
    pub target_id:        String,
    pub evidence_id:      String,
    pub filename:         String,
    pub category:         String,
    pub description:      Option<String>,
    pub confidence_score: i32,
    pub stored_phash:     String,
    /// Raw Hamming distance (0 = identical, 64 = completely different)
    pub hamming_distance: u32,
    /// Human-friendly 0–100 score (100 = identical)
    pub confidence_pct:   i32,
    pub created_at:       i64,
    // Denormalised from evidence table
    pub evidence_number:  String,
    pub uploader_id:      String,
    pub uploader_email:   String,
}

/// Returned by `search_face_encodings()` — one entry per encoding that is
/// within the threshold distance of the query descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMatchResult {
    /// The face_encodings row that matched
    pub encoding_id:      String,
    pub target_id:        String,
    pub evidence_id:      String,
    pub face_index:       i32,
    /// Raw Euclidean distance (lower = more similar).  0.0 = identical.
    pub distance:         f64,
    /// Human-friendly 0–100 score derived from distance vs threshold.
    pub confidence_score: i32,
    pub detection_score:  f64,
    pub phash:            Option<String>,
    pub auto_generated:   bool,
    pub created_at:       i64,
    // Denormalised from targets + evidence for convenience
    pub category:         String,
    pub description:      String,
    pub evidence_number:  String,
    pub uploader_id:      String,
    pub uploader_email:   String,
}