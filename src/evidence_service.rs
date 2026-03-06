// src/evidence_service.rs - CORRECTED VERSION
use anyhow::{Result, Context};
use chrono::{DateTime, Utc, NaiveDate, NaiveTime};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use web3::signing::keccak256;
use hex;
use rand::Rng;
use base64::prelude::*;

use crate::models::*;
use crate::storj::StorjService;
use crate::auth::AuthService;
use crate::Database;
use crate::models::ChartData;
use crate::models::{MediaStorageStats, StorageSizeData};
use crate::face_client::FaceClient;
use serde_json;

#[derive(Debug, Clone)]
pub struct EvidenceService {
    pub database: Database,
    storj_service: StorjService,
    pub face_client: FaceClient,
}

impl EvidenceService {
    pub fn new(storj_service: StorjService, database: Database) -> Self {
        Self {
            database,
            storj_service,
            face_client: FaceClient::new(),
        }
    }

    /// Extract country from evidence form data.
    /// Priority: 1) form.country from RobustGeolocation JS, 2) uploader profile, 3) county heuristic
    fn extract_country_from_evidence(form: &EvidenceForm, uploader: &SessionUser) -> String {
        // TIER 1: Use the country the JS geolocation already resolved
        if let Some(ref country) = form.country {
            if !country.is_empty() {
                println!("🌍 Country from geolocation: {}", country);
                return country.clone();
            }
        }

        // TIER 2: Fall back to user profile county (Kenya heuristic)
        if let Some(ref county) = uploader.county {
            if !county.is_empty() {
                return "Kenya".to_string();
            }
        }

        // TIER 3: Fall back to form county field (Kenya heuristic)
        if !form.county.is_empty() {
            return "Kenya".to_string();
        }

        "Unknown".to_string()
    }

    // Generate evidence number (EVD-YYYY-XXXXX)
    fn generate_evidence_number(&self) -> String {
        let year = Utc::now().format("%Y").to_string();
        let mut rng = rand::rng();
        let sequence = format!("{:05}", rng.random_range(1..=99999));
        format!("EVD-{}-{}", year, sequence)
    }

    // Create new evidence record
    pub async fn create_evidence(
        &self,
        form: EvidenceForm,
        uploader: &SessionUser,
        files: Vec<(Vec<u8>, String, String)>, // (data, filename, mime_type)
        sign_with_wallet: bool,
        auth_service: &AuthService,
    ) -> Result<Evidence> {
        println!("📝 CREATE_EVIDENCE: Starting evidence creation for {}", uploader.email);
        
        // Validate emergency level
        let emergency_level = match form.emergency_level.to_lowercase().as_str() {
            "red" => EmergencyLevel::Red,
            "orange" => EmergencyLevel::Orange,
            "yellow" => EmergencyLevel::Yellow,
            "blue" => EmergencyLevel::Blue,
            _ => {
                println!("❌ Invalid emergency level: {}", form.emergency_level);
                return Err(anyhow::anyhow!("Invalid emergency level"));
            }
        };
        
        // Validate incident type
        let incident_type = match form.incident_type.as_str() {
            "HitAndRun" => IncidentType::HitAndRun,
            "Assault" => IncidentType::Assault,
            "ThreatToLife" => IncidentType::ThreatToLife,
            "PropertyDamage" => IncidentType::PropertyDamage,
            "Theft" => IncidentType::Theft,
            "Other" => IncidentType::Other,
            _ => {
                println!("❌ Invalid incident type: {}", form.incident_type);
                return Err(anyhow::anyhow!("Invalid incident type"));
            }
        };
        
        // Parse incident time
        let incident_date = NaiveDate::parse_from_str(&form.incident_date, "%Y-%m-%d")
            .map_err(|_e| {
                println!("❌ Failed to parse date: {}", _e);
                anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD")
            })?;
        
        let incident_time = NaiveTime::parse_from_str(&form.incident_time, "%H:%M")
            .map_err(|_e| {
                println!("❌ Failed to parse time: {}", _e);
                anyhow::anyhow!("Invalid time format. Use HH:MM")
            })?;
        
        let incident_datetime = incident_date.and_time(incident_time);
        let incident_time_utc = DateTime::<Utc>::from_naive_utc_and_offset(incident_datetime, Utc);
        
        // Validate location coordinates (MUST have GPS)
        if form.latitude.is_none() && form.longitude.is_none() {
            println!("❌ Missing GPS coordinates");
            return Err(anyhow::anyhow!("Location coordinates are required. Please enable GPS or enter manually."));
        }
        
        // Validate Kenya county
        if form.county.is_empty() {
            println!("❌ Missing county");
            return Err(anyhow::anyhow!("County is required"));
        }
        
        // Validate files (max 3)
        if files.is_empty() {
            println!("❌ No evidence files provided");
            return Err(anyhow::anyhow!("At least one evidence file is required"));
        }
        
        if files.len() > 3 {
            println!("❌ Too many files: {}", files.len());
            return Err(anyhow::anyhow!("Maximum 3 files allowed"));
        }
        
        // Generate evidence ID
        let evidence_id = format!("evidence_{}", Uuid::new_v4());
        let evidence_number = self.generate_evidence_number();
        
        println!("📁 EVIDENCE: Creating {} - {}", evidence_number, form.title);
        println!("   Emergency: {:?}", emergency_level);
        println!("   Incident: {:?}", incident_type);
        println!("   Location: {}, {:?}", form.county, form.latitude);
        
        // Detect country for directory structure
        let country = Self::extract_country_from_evidence(&form, uploader);
        println!("   Country: {}", country);
        
        // Upload files to Storj
        let mut media_files = Vec::new();
        let mut storj_urls = Vec::new();
        let mut total_size = 0u64;
        let mut mime_types = Vec::new();
        let mut hash_values = Vec::new();
        
        // In create_evidence method, when processing files:
        for (file_idx, (file_data, filename, mime_type)) in files.into_iter().enumerate() {
            println!("📤 Uploading file {}: {} ({:.2} MB, MIME: {})", 
                file_idx + 1,
                filename, 
                file_data.len() as f64 / (1024.0 * 1024.0),
                mime_type);
            
            // Fix MIME type for live recordings if needed
            let corrected_mime_type = if filename.contains("live_recording") || filename.contains("recording_") {
                if mime_type.is_empty() || mime_type == "application/octet-stream" {
                    if filename.ends_with(".webm") {
                        "video/webm".to_string()
                    } else if filename.ends_with(".mp4") {
                        "video/mp4".to_string()
                    } else {
                        "video/webm".to_string() // Default for live recordings
                    }
                } else {
                    mime_type
                }
            } else {
                mime_type
            };
            
            // Generate hash
            let mut hasher = Sha256::new();
            hasher.update(&file_data);
            let file_hash = hex::encode(hasher.finalize());
            hash_values.push(file_hash.clone());
            
            // Upload to Storj with country-based path
            let upload_result = self.storj_service.upload_bytes_with_public_access_country(
                &file_data,
                &filename,
                &corrected_mime_type, // Use corrected MIME type
                &country, // Pass country for directory structure
            ).await.context("Failed to upload to Storj")?;
            
            println!("✅ Uploaded: {}", upload_result.public_url);
            println!("   Corrected MIME type: {}", corrected_mime_type);
            
            // Create media file record
            let media_id = format!("media_{}_{}", Uuid::new_v4(), file_idx);
            let media_file = MediaFile {
                id: media_id,
                filename: filename.clone(),
                mime_type: corrected_mime_type.clone(), // Store corrected MIME type
                file_size: file_data.len() as u64,
                duration_seconds: None, // Could extract from video files
                thumbnail_url: None,
                storj_url: upload_result.public_url.clone(),
                storj_key: upload_result.key,
                hash: file_hash,
                quality_rating: 3, // Default
                description: None,
            };
            
            media_files.push(media_file);
            storj_urls.push(upload_result.public_url);
            mime_types.push(corrected_mime_type); // Store corrected MIME type
            total_size += file_data.len() as u64;
        }
        
        // Create location
        let location = EvidenceLocation {
            county: form.county,
            constituency: form.constituency,
            ward: form.ward,
            latitude: form.latitude.unwrap_or(0.0),
            longitude: form.longitude.unwrap_or(0.0),
            landmark: form.landmark,
            address: None,
        };
        
        // Create vehicle details if hit & run
        let vehicle_details = if incident_type == IncidentType::HitAndRun {
            Some(VehicleDetails {
                registration: form.vehicle_registration,
                color: form.vehicle_color,
                vehicle_type: form.vehicle_type.and_then(|vt| match vt.as_str() {
                    "Matatu" => Some(VehicleType::Matatu),
                    "BodaBoda" => Some(VehicleType::BodaBoda),
                    "Private" => Some(VehicleType::Private),
                    "PSV" => Some(VehicleType::PSV),
                    "Lorry" => Some(VehicleType::Lorry),
                    "Taxi" => Some(VehicleType::Taxi),
                    _ => Some(VehicleType::Unknown),
                }),
                make_model: None,
                sacco_name: form.sacco_name,
                description: None,
            })
        } else {
            None
        };
        
        // Generate evidence hash for signing
        let evidence_hash = self.generate_evidence_hash(&evidence_id, &incident_time_utc, &location);
        let mut wallet_signature = None;
        let mut signature_timestamp = None;
        
        // Wallet signing if requested
        if sign_with_wallet {
            if let (Some(wallet_addr), Some(chain)) = (&uploader.wallet_address, &uploader.wallet_chain) {
                println!("🔐 Signing evidence with wallet: {}", wallet_addr);
                
                match auth_service.sign_evidence_hash(
                    &evidence_hash,
                    &evidence_id,
                    wallet_addr,
                    chain,
                ).await {
                    Ok(signature) => {
                        wallet_signature = Some(signature.signature.clone());
                        signature_timestamp = Some(signature.timestamp);
                        
                        // Store signature in database
                        self.database.store_evidence_signature(&signature).await?;
                        println!("✅ Evidence signed successfully");
                    }
                    Err(e) => {
                        println!("⚠️ Failed to sign evidence: {}", e);
                        // Continue without signature
                    }
                }
            } else {
                println!("⚠️ No wallet connected, skipping signature");
            }
        }
        
        // Calculate evidence quality (simple heuristic)
        let evidence_quality = self.calculate_evidence_quality(
            &media_files,
            &location,
            &vehicle_details,
            &form.description,
        );
        
        // Create evidence record
        let now = Utc::now();
        let evidence = Evidence {
            // Core Identification
            id: evidence_id.clone(),
            evidence_number: evidence_number.clone(),
            
            // Incident Classification
            emergency_level: emergency_level.clone(),
            incident_type,
            sub_type: form.sub_type,
            
            // Time & Location
            incident_time: incident_time_utc,
            report_time: now,
            location,
            
            // Vehicle Details
            vehicle_details,
            
            // Description
            title: form.title,
            description: form.description,
            injuries: form.injuries,
            property_damage: form.property_damage,
            suspect_description: form.suspect_description,
            
            // Uploader Info
            uploader_id: uploader.id.clone(),
            uploader_email: uploader.email.clone(),
            uploader_phone: uploader.phone_number.clone(),
            
            // Evidence Media
            media_files,
            evidence_quality,
            
            // Police Integration
            reported_to_police: form.reported_to_police,
            police_case_id: form.police_case_id,
            police_station: form.police_station,
            report_date: if form.reported_to_police { Some(now) } else { None },
            
            // Blockchain Verification
            wallet_signature,
            wallet_address: uploader.wallet_address.clone(),
            signature_timestamp,
            
            // Status & Metadata
            status: if form.reported_to_police {
                EvidenceStatus::Reported
            } else {
                EvidenceStatus::Submitted
            },
            needs_attention: evidence_quality < 3 || emergency_level == EmergencyLevel::Red,
            is_anonymous: form.is_anonymous,
            chain_of_custody: vec![CustodyRecord {
                action: "created".to_string(),
                user_id: uploader.id.clone(),
                timestamp: now,
                details: "Evidence record created".to_string(),
            }],
            
            // Technical
            storj_urls,
            storj_bucket: Some(self.storj_service.bucket_name().to_string()),
            file_size_bytes: total_size,
            mime_types,
            hash_values,
            
            // Timestamps
            created_at: now,
            updated_at: now,
            reviewed_at: None,
        };
        
        // Save to database
        self.database.create_evidence(&evidence).await?;
        
        // Log in audit
        self.database.log_audit(
            Some(&uploader.id),
            "evidence_created",
            "evidence",
            Some(&evidence.id),
            &format!("Evidence created: {} - {}", evidence.evidence_number, evidence.title),
            None,
        ).await?;
        
        println!("✅ EVIDENCE CREATED: {}", evidence.evidence_number);
        println!("   ID: {}", evidence.id);
        println!("   Type: {:?}", evidence.incident_type);
        println!("   Emergency: {:?}", evidence.emergency_level);
        println!("   Location: {}, {}", evidence.location.county, evidence.location.latitude);
        println!("   Files: {}", evidence.media_files.len());
        println!("   Signed: {}", evidence.wallet_signature.is_some());
        println!("   Police Case: {:?}", evidence.police_case_id);
        
        Ok(evidence)
    }
    
    fn generate_evidence_hash(&self, evidence_id: &str, incident_time: &DateTime<Utc>, location: &EvidenceLocation) -> Vec<u8> {
        let data = format!(
            "FLUG_EVIDENCE_V1\nID: {}\nTime: {}\nLocation: {},{} ({})\nTimestamp: {}",
            evidence_id,
            incident_time.timestamp(),
            location.latitude,
            location.longitude,
            location.county,
            Utc::now().timestamp()
        );
        
        keccak256(data.as_bytes()).to_vec()
    }
    
    fn calculate_evidence_quality(
        &self,
        media_files: &[MediaFile],
        location: &EvidenceLocation,
        vehicle_details: &Option<VehicleDetails>,
        description: &str,
    ) -> i32 {
        let mut score = 3; // Default
        
        // Media quality
        if !media_files.is_empty() {
            score += 1;
            
            // Check if any video exists
            let has_video = media_files.iter().any(|f| f.mime_type.starts_with("video/"));
            if has_video {
                score += 1;
            }
            
            // Check if multiple angles
            if media_files.len() > 1 {
                score += 1;
            }
        }
        
        // Location completeness
        if location.latitude != 0.0 && location.longitude != 0.0 {
            score += 1;
        }
        
        if !location.county.is_empty() {
            score += 1;
        }
        
        if location.landmark.is_some() {
            score += 1;
        }
        
        // Vehicle details (for hit & run)
        if let Some(vehicle) = vehicle_details {
            if vehicle.registration.is_some() {
                score += 2;
            }
            
            if vehicle.color.is_some() {
                score += 1;
            }
            
            if vehicle.vehicle_type.is_some() {
                score += 1;
            }
        }
        
        // Description quality
        if description.len() > 100 {
            score += 1;
        }
        
        if description.len() > 500 {
            score += 1;
        }
        
        // Cap at 5
        score.min(5).max(1)
    }
    
    // Get evidence by ID
    pub async fn get_evidence(&self, evidence_id: &str, increment_views: bool) -> Result<Option<Evidence>> {
        self.database.get_evidence(evidence_id, increment_views).await
    }
    
    // Get user's evidence
    pub async fn get_user_evidence(&self, user_id: &str) -> Result<Vec<EvidenceSummary>> {
        self.database.get_user_evidence(user_id).await
    }
    
    // Update evidence (add police case ID, etc.)
    pub async fn update_evidence(
        &self,
        evidence_id: &str,
        user_id: &str,
        updates: EvidenceUpdate,
    ) -> Result<Evidence> {
        // Get current evidence
        let mut evidence = match self.database.get_evidence(evidence_id, false).await? {
            Some(evidence) => evidence,
            None => return Err(anyhow::anyhow!("Evidence not found")),
        };
        
        // Verify ownership
        if evidence.uploader_id != user_id {
            return Err(anyhow::anyhow!("You don't own this evidence"));
        }
        
        // Apply updates
        if let Some(police_case_id) = updates.police_case_id {
            evidence.police_case_id = Some(police_case_id.clone());
            evidence.reported_to_police = true;
            evidence.report_date = Some(Utc::now());
            evidence.status = EvidenceStatus::Reported;
            
            // Add to chain of custody
            let now = Utc::now();
            evidence.chain_of_custody.push(CustodyRecord {
                action: "police_report_added".to_string(),
                user_id: user_id.to_string(),
                timestamp: now,
                details: format!("Added police case ID: {}", police_case_id),
            });
        }
        
        // Update other fields
        if let Some(police_station) = updates.police_station {
            evidence.police_station = Some(police_station);
        }
        
        if let Some(reported_to_police) = updates.reported_to_police {
            evidence.reported_to_police = reported_to_police;
        }
        
        if let Some(status) = updates.status {
            evidence.status = status;
        }
        
        if let Some(needs_attention) = updates.needs_attention {
            evidence.needs_attention = needs_attention;
        }
        
        if let Some(description) = updates.description {
            evidence.description = description;
        }
        
        if let Some(vehicle_details) = updates.vehicle_details {
            evidence.vehicle_details = Some(vehicle_details);
        }
        
        evidence.updated_at = Utc::now();
        
        // Save updates
        self.database.update_evidence(&evidence).await?;
        
        // Log update
        self.database.log_audit(
            Some(user_id),
            "evidence_updated",
            "evidence",
            Some(evidence_id),
            "Evidence updated",
            None,
        ).await?;
        
        Ok(evidence)
    }
    
    // Get dashboard statistics
    pub async fn get_dashboard_stats(&self, user_id: &str) -> Result<DashboardStats> {
        self.database.get_evidence_stats(user_id).await
    }
    
    // Sign existing evidence
    pub async fn sign_evidence(
        &self,
        evidence_id: &str,
        user_id: &str,
        auth_service: &AuthService,
    ) -> Result<EvidenceSignature> {
        // Get evidence
        let evidence = match self.database.get_evidence(evidence_id, false).await? {
            Some(evidence) => evidence,
            None => return Err(anyhow::anyhow!("Evidence not found")),
        };
        
        // Verify ownership
        if evidence.uploader_id != user_id {
            return Err(anyhow::anyhow!("You don't own this evidence"));
        }
        
        // Check if already signed
        if evidence.wallet_signature.is_some() {
            return Err(anyhow::anyhow!("Evidence already signed"));
        }
        
        // Get user wallet
        let user = match auth_service.get_session_user(&evidence.uploader_email).await? {
            Some(user) => user,
            None => return Err(anyhow::anyhow!("User not found")),
        };
        
        if !user.has_wallet {
            return Err(anyhow::anyhow!("No wallet connected"));
        }
        
        let (wallet_addr, chain) = match (&user.wallet_address, &user.wallet_chain) {
            (Some(addr), Some(chain)) => (addr, chain),
            _ => return Err(anyhow::anyhow!("Wallet information missing")),
        };
        
        // Generate hash and sign
        let evidence_hash = self.generate_evidence_hash(&evidence.id, &evidence.incident_time, &evidence.location);
        
        let signature = auth_service.sign_evidence_hash(
            &evidence_hash,
            evidence_id,
            wallet_addr,
            chain,
        ).await?;
        
        // Update evidence with signature
        let mut updated_evidence = evidence.clone();
        updated_evidence.wallet_signature = Some(signature.signature.clone());
        updated_evidence.wallet_address = Some(wallet_addr.clone());
        updated_evidence.signature_timestamp = Some(signature.timestamp);
        updated_evidence.updated_at = Utc::now();
        
        // Add to chain of custody
        let now = Utc::now();
        updated_evidence.chain_of_custody.push(CustodyRecord {
            action: "signed".to_string(),
            user_id: user_id.to_string(),
            timestamp: now,
            details: "Evidence signed with blockchain wallet".to_string(),
        });
        
        self.database.update_evidence(&updated_evidence).await?;
        
        println!("✅ Evidence signed: {}", evidence_id);
        Ok(signature)
    }
    
    // Generate QR code for evidence sharing
    pub async fn generate_evidence_qr(&self, evidence_id: &str) -> Result<String> {
        // In production, generate actual QR code
        // For now, return a URL
        Ok(format!("https://flugevidence.ke/evidence/{}", evidence_id))
    }
    
    // In evidence_service.rs, update the search_evidence_with_filters method:

    pub async fn search_evidence_with_filters(
        &self,
        filters: &EvidenceSearchFilters,
        current_user_id: &str,
    ) -> Result<EvidenceSearchResponse> {
        println!("🔍 SEARCH_EVIDENCE: Searching with filters");
        println!("   User ID: {}", current_user_id);
        println!("   Show only mine: {:?}", filters.uploader_id);
        
        // Get all evidence
        let all_evidence = self.database.get_all_evidence().await?;
        println!("🔍 SEARCH_EVIDENCE: Total evidence in system: {}", all_evidence.len());
        
        // Apply filters
        let mut filtered_evidence = Vec::new();
        let mut filtered_summaries = Vec::new();
        
        for evidence in all_evidence {
            println!("🔍 Processing evidence: {}", evidence.evidence_number);
            
            // Skip if evidence is not owned by current user and user only wants their own
            if filters.uploader_id.as_ref().map_or(false, |uid| uid != &evidence.uploader_id) {
                println!("   Skipping - not owned by current user");
                continue;
            }
            
            // Apply query filter if provided
            if let Some(query) = &filters.query {
                if !query.is_empty() {
                    let search_text = query.to_lowercase();
                    let matches = evidence.title.to_lowercase().contains(&search_text) ||
                                evidence.description.to_lowercase().contains(&search_text) ||
                                evidence.evidence_number.to_lowercase().contains(&search_text) ||
                                evidence.location.county.to_lowercase().contains(&search_text) ||
                                evidence.uploader_email.to_lowercase().contains(&search_text);
                    if !matches {
                        println!("   Skipping - doesn't match query");
                        continue;
                    }
                }
            }
            
            // Apply incident type filter
            if let Some(incident_type) = &filters.incident_type {
                if !incident_type.is_empty() {
                    let evidence_type_str = match evidence.incident_type {
                        IncidentType::HitAndRun => "HitAndRun",
                        IncidentType::Assault => "Assault",
                        IncidentType::ThreatToLife => "ThreatToLife",
                        IncidentType::PropertyDamage => "PropertyDamage",
                        IncidentType::Theft => "Theft",
                        IncidentType::Other => "Other",
                    };
                    if evidence_type_str != incident_type {
                        println!("   Skipping - incident type doesn't match");
                        continue;
                    }
                }
            }
            
            // Apply county filter
            if let Some(county) = &filters.county {
                if !county.is_empty() && evidence.location.county != *county {
                    println!("   Skipping - county doesn't match");
                    continue;
                }
            }
            
            // Apply emergency level filter
            if let Some(emergency_level) = &filters.emergency_level {
                if !emergency_level.is_empty() {
                    let evidence_level_str = match evidence.emergency_level {
                        EmergencyLevel::Red => "red",
                        EmergencyLevel::Orange => "orange",
                        EmergencyLevel::Yellow => "yellow",
                        EmergencyLevel::Blue => "blue",
                    };
                    if evidence_level_str != emergency_level {
                        println!("   Skipping - emergency level doesn't match");
                        continue;
                    }
                }
            }
            
            // Apply status filter
            if let Some(status) = &filters.status {
                if !status.is_empty() {
                    let evidence_status_str = match evidence.status {
                        EvidenceStatus::Draft => "draft",
                        EvidenceStatus::Submitted => "submitted",
                        EvidenceStatus::Reported => "reported",
                        EvidenceStatus::UnderReview => "under_review",
                        EvidenceStatus::Archived => "archived",
                        EvidenceStatus::Rejected => "rejected",
                    };
                    if evidence_status_str != status {
                        println!("   Skipping - status doesn't match");
                        continue;
                    }
                }
            }
            
            // Apply reported_to_police filter
            if let Some(reported) = filters.reported_to_police {
                if evidence.reported_to_police != reported {
                    println!("   Skipping - reported status doesn't match");
                    continue;
                }
            }
            
            // Apply needs_attention filter
            if let Some(needs_attention) = filters.needs_attention {
                if evidence.needs_attention != needs_attention {
                    println!("   Skipping - needs attention doesn't match");
                    continue;
                }
            }
            
            // Apply signed_only filter
            if let Some(signed_only) = filters.signed_only {
                if signed_only && evidence.wallet_signature.is_none() {
                    println!("   Skipping - not signed but signed_only filter is on");
                    continue;
                }
            }
            
            // Date filtering
            if let Some(date_from) = &filters.date_from {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(date_from, "%Y-%m-%d") {
                    let datetime = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
                    if evidence.incident_time < datetime {
                        println!("   Skipping - date before filter");
                        continue;
                    }
                }
            }
            
            if let Some(date_to) = &filters.date_to {
                if let Ok(date) = chrono::NaiveDate::parse_from_str(date_to, "%Y-%m-%d") {
                    let datetime = date.and_hms_opt(23, 59, 59).unwrap().and_utc();
                    if evidence.incident_time > datetime {
                        println!("   Skipping - date after filter");
                        continue;
                    }
                }
            }
            
            // Add to results
            filtered_evidence.push(evidence.clone());
            
            // Create summary
            let summary = EvidenceSummary {
                id: evidence.id.clone(),
                evidence_number: evidence.evidence_number.clone(),
                emergency_level: evidence.emergency_level.clone(),
                incident_type: evidence.incident_type.clone(),
                title: evidence.title.clone(),
                county: evidence.location.county.clone(),
                incident_time: evidence.incident_time,
                status: evidence.status.clone(),
                reported_to_police: evidence.reported_to_police,
                police_case_id: evidence.police_case_id.clone(),
                has_media: !evidence.media_files.is_empty(),
                needs_attention: evidence.needs_attention,
            };
            filtered_summaries.push(summary);
            
            println!("   ✅ Added to results");
        }
        
        println!("🔍 SEARCH_EVIDENCE: Filtered down to {} items", filtered_evidence.len());
        
        // Apply sorting
        let sort_by = filters.sort_by.as_deref().unwrap_or("newest");
        match sort_by {
            "newest" => {
                filtered_evidence.sort_by(|a, b| b.incident_time.cmp(&a.incident_time));
                filtered_summaries.sort_by(|a, b| b.incident_time.cmp(&a.incident_time));
            }
            "oldest" => {
                filtered_evidence.sort_by(|a, b| a.incident_time.cmp(&b.incident_time));
                filtered_summaries.sort_by(|a, b| a.incident_time.cmp(&b.incident_time));
            }
            "urgent" => {
                filtered_evidence.sort_by(|a, b| {
                    let a_priority = match a.emergency_level {
                        EmergencyLevel::Red => 4,
                        EmergencyLevel::Orange => 3,
                        EmergencyLevel::Yellow => 2,
                        EmergencyLevel::Blue => 1,
                    };
                    let b_priority = match b.emergency_level {
                        EmergencyLevel::Red => 4,
                        EmergencyLevel::Orange => 3,
                        EmergencyLevel::Yellow => 2,
                        EmergencyLevel::Blue => 1,
                    };
                    b_priority.cmp(&a_priority).then(b.incident_time.cmp(&a.incident_time))
                });
                filtered_summaries.sort_by(|a, b| {
                    let a_priority = match a.emergency_level {
                        EmergencyLevel::Red => 4,
                        EmergencyLevel::Orange => 3,
                        EmergencyLevel::Yellow => 2,
                        EmergencyLevel::Blue => 1,
                    };
                    let b_priority = match b.emergency_level {
                        EmergencyLevel::Red => 4,
                        EmergencyLevel::Orange => 3,
                        EmergencyLevel::Yellow => 2,
                        EmergencyLevel::Blue => 1,
                    };
                    b_priority.cmp(&a_priority).then(b.incident_time.cmp(&a.incident_time))
                });
            }
            _ => {
                // Default to newest
                filtered_evidence.sort_by(|a, b| b.incident_time.cmp(&a.incident_time));
                filtered_summaries.sort_by(|a, b| b.incident_time.cmp(&a.incident_time));
            }
        }
        
        // Apply pagination
        let page = filters.page;
        let limit = filters.limit;
        
        let start_index = (page - 1) as usize * limit as usize;
        let end_index = std::cmp::min(start_index + limit as usize, filtered_evidence.len());
        
        let paginated_evidence = if start_index < filtered_evidence.len() {
            filtered_evidence[start_index..end_index].to_vec()
        } else {
            Vec::new()
        };
        
        let paginated_summaries = if start_index < filtered_summaries.len() {
            filtered_summaries[start_index..end_index].to_vec()
        } else {
            Vec::new()
        };
        
        // Calculate total pages using integer ceiling division
        let total_items = filtered_evidence.len() as u32;
        let total_pages = if limit > 0 {
            std::cmp::max((total_items + limit - 1) / limit, 1)
        } else {
            1
        };
        
        println!("🔍 SEARCH_EVIDENCE: Final result - {} items, page {}/{}", 
                filtered_evidence.len(), page, total_pages);
        
        Ok(EvidenceSearchResponse {
            evidence: paginated_evidence,
            summaries: paginated_summaries,
            total: filtered_evidence.len() as u64,
            page: page,
            total_pages: total_pages,
        })
    }
            
    // Media search (for MediaSearchFilters)
    pub async fn search_media_evidence(
        &self,
        _filters: &MediaSearchFilters,
        _current_user_id: &str,
    ) -> Result<MediaSearchResponse> {
        // This is for media content search, not evidence
        Ok(MediaSearchResponse {
            content: Vec::new(),
            total: 0,
            page: 1,
            total_pages: 1,
        })
    }


    /// Create draft evidence (media only)
    pub async fn create_draft_evidence(
        &self,
        form: EvidenceForm,
        uploader: &SessionUser,
        files: Vec<(Vec<u8>, String, String)>,
    ) -> Result<Evidence> {
        println!("📝 CREATE_DRAFT_EVIDENCE: Creating draft evidence for {}", uploader.email);
        
        // Use default values for draft
        let emergency_level = EmergencyLevel::Blue; // Default to low
        let incident_type = IncidentType::Other; // Default type
        
        // Parse dates (use current time for draft)
        let now = Utc::now();
        let _incident_datetime = now.naive_utc();
        let incident_time_utc = now;
        
        // Validate files
        if files.is_empty() {
            println!("❌ No evidence files provided");
            return Err(anyhow::anyhow!("At least one evidence file is required"));
        }
        
        if files.len() > 3 {
            println!("❌ Too many files: {}", files.len());
            return Err(anyhow::anyhow!("Maximum 3 files allowed"));
        }
        
        // Generate evidence ID and number
        let evidence_id = format!("evidence_{}", Uuid::new_v4());
        let evidence_number = self.generate_evidence_number();
        
        // Resolve country before uploading so files land in the right folder
        let country = Self::extract_country_from_evidence(&form, uploader);
        println!("   Country: {}", country);
        
        println!("📁 DRAFT_EVIDENCE: Creating {} - {}", evidence_number, form.title);
        println!("   Files: {}", files.len());
        
        // Upload files to Storj (same as before)
        let mut media_files = Vec::new();
        let mut storj_urls = Vec::new();
        let mut total_size = 0u64;
        let mut mime_types = Vec::new();
        let mut hash_values = Vec::new();
        
        for (file_idx, (file_data, filename, mime_type)) in files.into_iter().enumerate() {
            println!("📤 Uploading file {}: {} ({:.2} MB)", 
                file_idx + 1,
                filename, 
                file_data.len() as f64 / (1024.0 * 1024.0));
            
            // Generate hash
            let mut hasher = Sha256::new();
            hasher.update(&file_data);
            let file_hash = hex::encode(hasher.finalize());
            hash_values.push(file_hash.clone());
            
            // Upload to Storj — pass country so file lands in correct folder
            let upload_result = self.storj_service.upload_bytes_with_public_access_country(
                &file_data,
                &filename,
                &mime_type,
                &country,
            ).await.context("Failed to upload to Storj")?;
            
            println!("✅ Uploaded: {}", upload_result.public_url);
            
            // Create media file record
            let media_id = format!("media_{}_{}", Uuid::new_v4(), file_idx);
            let media_file = MediaFile {
                id: media_id,
                filename: filename.clone(),
                mime_type: mime_type.clone(),
                file_size: file_data.len() as u64,
                duration_seconds: None,
                thumbnail_url: None,
                storj_url: upload_result.public_url.clone(),
                storj_key: upload_result.key,
                hash: file_hash,
                quality_rating: 3,
                description: None,
            };
            
            media_files.push(media_file);
            storj_urls.push(upload_result.public_url);
            mime_types.push(mime_type);
            total_size += file_data.len() as u64;
        }
        
        // Create location with placeholder
        let location = EvidenceLocation {
            county: form.county,
            constituency: None, // Empty for draft
            ward: None,
            latitude: 0.0, // Placeholder
            longitude: 0.0, // Placeholder
            landmark: None,
            address: None,
        };
        
        // Create evidence record with DRAFT status
        let evidence = Evidence {
            // Core Identification
            id: evidence_id.clone(),
            evidence_number: evidence_number.clone(),
            
            // Incident Classification
            emergency_level,
            incident_type,
            sub_type: None, // Empty for draft
            
            // Time & Location
            incident_time: incident_time_utc,
            report_time: now,
            location,
            
            // Vehicle Details (none for draft)
            vehicle_details: None,
            
            // Description
            title: form.title,
            description: form.description,
            injuries: None, // Empty for draft
            property_damage: None,
            suspect_description: None,
            
            // Uploader Info
            uploader_id: uploader.id.clone(),
            uploader_email: uploader.email.clone(),
            uploader_phone: uploader.phone_number.clone(),
            
            // Evidence Media
            media_files,
            evidence_quality: 1, // Low quality until completed
            
            // Police Integration (none for draft)
            reported_to_police: false,
            police_case_id: None,
            police_station: None,
            report_date: None,
            
            // Blockchain Verification (none for draft)
            wallet_signature: None,
            wallet_address: None,
            signature_timestamp: None,
            
            // Status & Metadata - SET TO DRAFT
            status: EvidenceStatus::Draft,
            needs_attention: true, // Drafts need attention
            is_anonymous: false,
            chain_of_custody: vec![CustodyRecord {
                action: "draft_created".to_string(),
                user_id: uploader.id.clone(),
                timestamp: now,
                details: "Draft evidence created with media files only".to_string(),
            }],
            
            // Technical
            storj_urls,
            storj_bucket: Some(self.storj_service.bucket_name().to_string()),
            file_size_bytes: total_size,
            mime_types,
            hash_values,
            
            // Timestamps
            created_at: now,
            updated_at: now,
            reviewed_at: None,
        };
        
        // Save to database
        self.database.create_evidence(&evidence).await?;
        
        // Log in audit
        self.database.log_audit(
            Some(&uploader.id),
            "draft_evidence_created",
            "evidence",
            Some(&evidence.id),
            &format!("Draft evidence created: {} ({} files)", evidence.evidence_number, evidence.media_files.len()),
            None,
        ).await?;
        
        println!("✅ DRAFT_EVIDENCE CREATED: {}", evidence.evidence_number);
        println!("   ID: {}", evidence.id);
        println!("   Status: {:?}", evidence.status);
        println!("   Files: {}", evidence.media_files.len());
        
        Ok(evidence)
    }

    /// Complete draft evidence with full details
    pub async fn complete_draft_evidence(
        &self,
        evidence_id: &str,
        form: EvidenceForm,
        user_id: &str,
        auth_service: &AuthService,
        sign_with_wallet: bool,
    ) -> Result<Evidence> {
        println!("📝 COMPLETE_DRAFT_EVIDENCE: Completing draft {}", evidence_id);
        
        // Get current draft evidence
        let mut evidence = match self.database.get_evidence(evidence_id, false).await? {
            Some(evidence) => evidence,
            None => return Err(anyhow::anyhow!("Evidence not found")),
        };
        
        // Verify ownership
        if evidence.uploader_id != user_id {
            return Err(anyhow::anyhow!("You don't own this evidence"));
        }
        
        // Verify it's a draft
        if evidence.status != EvidenceStatus::Draft {
            return Err(anyhow::anyhow!("Evidence is not a draft"));
        }
        
        // Validate emergency level
        let emergency_level = match form.emergency_level.to_lowercase().as_str() {
            "red" => EmergencyLevel::Red,
            "orange" => EmergencyLevel::Orange,
            "yellow" => EmergencyLevel::Yellow,
            "blue" => EmergencyLevel::Blue,
            _ => {
                println!("❌ Invalid emergency level: {}", form.emergency_level);
                return Err(anyhow::anyhow!("Invalid emergency level"));
            }
        };
        
        // Validate incident type - store as value to clone later
        let incident_type = match form.incident_type.as_str() {
            "HitAndRun" => IncidentType::HitAndRun,
            "Assault" => IncidentType::Assault,
            "ThreatToLife" => IncidentType::ThreatToLife,
            "PropertyDamage" => IncidentType::PropertyDamage,
            "Theft" => IncidentType::Theft,
            "Other" => IncidentType::Other,
            _ => {
                println!("❌ Invalid incident type: {}", form.incident_type);
                return Err(anyhow::anyhow!("Invalid incident type"));
            }
        };
        
        // Parse incident time
        let incident_date = NaiveDate::parse_from_str(&form.incident_date, "%Y-%m-%d")
            .map_err(|_e| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?;
        
        let incident_time = NaiveTime::parse_from_str(&form.incident_time, "%H:%M")
            .map_err(|_e| anyhow::anyhow!("Invalid time format. Use HH:MM"))?;
        
        let incident_datetime = incident_date.and_time(incident_time);
        let incident_time_utc = DateTime::<Utc>::from_naive_utc_and_offset(incident_datetime, Utc);
        
        // Validate location coordinates
        if form.latitude.is_none() && form.longitude.is_none() {
            println!("❌ Missing GPS coordinates");
            return Err(anyhow::anyhow!("Location coordinates are required"));
        }
        
        // Validate Kenya county
        if form.county.is_empty() || form.county == "N/A" {
            println!("❌ Missing county");
            return Err(anyhow::anyhow!("County is required"));
        }
        
        // Update evidence with new data
        evidence.emergency_level = emergency_level;
        evidence.incident_type = incident_type.clone(); // Clone here
        evidence.sub_type = form.sub_type;
        evidence.incident_time = incident_time_utc;
        
        // Update location
        evidence.location = EvidenceLocation {
            county: form.county,
            constituency: form.constituency,
            ward: form.ward,
            latitude: form.latitude.unwrap_or(0.0),
            longitude: form.longitude.unwrap_or(0.0),
            landmark: form.landmark,
            address: None,
        };
        
        // Update vehicle details if hit & run - use the incident_type variable (not moved yet)
        evidence.vehicle_details = if incident_type == IncidentType::HitAndRun { // Use incident_type directly
            Some(VehicleDetails {
                registration: form.vehicle_registration,
                color: form.vehicle_color,
                vehicle_type: form.vehicle_type.and_then(|vt| match vt.as_str() {
                    "Matatu" => Some(VehicleType::Matatu),
                    "BodaBoda" => Some(VehicleType::BodaBoda),
                    "Private" => Some(VehicleType::Private),
                    "PSV" => Some(VehicleType::PSV),
                    "Lorry" => Some(VehicleType::Lorry),
                    "Taxi" => Some(VehicleType::Taxi),
                    _ => Some(VehicleType::Unknown),
                }),
                make_model: None,
                sacco_name: form.sacco_name,
                description: None,
            })
        } else {
            None
        };
        
        // Update description fields
        evidence.title = form.title;
        evidence.description = form.description;
        evidence.injuries = form.injuries;
        evidence.property_damage = form.property_damage;
        evidence.suspect_description = form.suspect_description;
        
        // Update police integration
        evidence.reported_to_police = form.reported_to_police;
        evidence.police_case_id = form.police_case_id;
        evidence.police_station = form.police_station;
        evidence.report_date = if form.reported_to_police { Some(Utc::now()) } else { None };
        
        // Recalculate evidence quality
        evidence.evidence_quality = self.calculate_evidence_quality(
            &evidence.media_files,
            &evidence.location,
            &evidence.vehicle_details,
            &evidence.description,
        );
        
        // Update status from DRAFT to SUBMITTED
        evidence.status = if form.reported_to_police {
            EvidenceStatus::Reported
        } else {
            EvidenceStatus::Submitted
        };
        
        evidence.needs_attention = evidence.evidence_quality < 3 || evidence.emergency_level == EmergencyLevel::Red;
        
        // Wallet signing if requested
        if sign_with_wallet {
            // Get session user from auth service to get wallet info
            match auth_service.get_session_user(&evidence.uploader_email).await {
                Ok(Some(session_user)) => {
                    if session_user.has_wallet {
                        if let (Some(wallet_addr), Some(wallet_chain)) = (&session_user.wallet_address, &session_user.wallet_chain) {
                            println!("🔐 Signing evidence with wallet: {}", wallet_addr);
                            
                            let evidence_hash = self.generate_evidence_hash(&evidence.id, &evidence.incident_time, &evidence.location);
                            
                            match auth_service.sign_evidence_hash(
                                &evidence_hash,
                                &evidence.id,
                                wallet_addr,
                                wallet_chain,
                            ).await {
                                Ok(signature) => {
                                    evidence.wallet_signature = Some(signature.signature.clone());
                                    evidence.wallet_address = Some(wallet_addr.clone());
                                    evidence.signature_timestamp = Some(signature.timestamp);
                                    
                                    // Store signature in database
                                    self.database.store_evidence_signature(&signature).await?;
                                    println!("✅ Evidence signed successfully");
                                }
                                Err(e) => {
                                    println!("⚠️ Failed to sign evidence: {}", e);
                                    // Continue without signature
                                }
                            }
                        } else {
                            println!("⚠️ User has wallet flag but no wallet address/chain");
                        }
                    } else {
                        println!("⚠️ User doesn't have wallet connected, skipping signature");
                    }
                }
                Ok(None) => {
                    println!("⚠️ Session user not found, skipping signature");
                }
                Err(e) => {
                    println!("⚠️ Error getting session user: {}, skipping signature", e);
                }
            }
        }
        
        // Add to chain of custody
        let now = Utc::now();
        evidence.chain_of_custody.push(CustodyRecord {
            action: "completed".to_string(),
            user_id: user_id.to_string(),
            timestamp: now,
            details: "Draft evidence completed with full details".to_string(),
        });
        
        evidence.updated_at = now;
        
        // Update evidence in database
        self.database.update_evidence(&evidence).await?;
        
        // Log in audit
        self.database.log_audit(
            Some(user_id),
            "draft_completed",
            "evidence",
            Some(evidence_id),
            &format!("Draft completed: {} - {}", evidence.evidence_number, evidence.title),
            None,
        ).await?;
        
        println!("✅ DRAFT COMPLETED: {}", evidence.evidence_number);
        println!("   New Status: {:?}", evidence.status);
        println!("   Title: {}", evidence.title);
        println!("   Location: {}", evidence.location.county);
        
        Ok(evidence)
    }

    /// Upload target photos for evidence
    pub async fn upload_targets(
        &self,
        request: TargetUploadRequest,
        user_id: &str,
    ) -> Result<Vec<TargetPhoto>> {
        println!("🎯 UPLOAD_TARGETS: Uploading targets for evidence: {}", request.evidence_id);
        
        // Verify evidence exists and user has permission
        let evidence = match self.database.get_evidence(&request.evidence_id, false).await? {
            Some(evidence) => evidence,
            None => {
                println!("❌ Evidence not found: {}", request.evidence_id);
                return Err(anyhow::anyhow!("Evidence not found"));
            }
        };
        
        // Verify ownership
        if evidence.uploader_id != user_id {
            println!("❌ User {} doesn't own evidence {}", user_id, request.evidence_id);
            return Err(anyhow::anyhow!("You don't have permission to upload targets for this evidence"));
        }
        
        // Validate target count
        if request.photos.len() > 15 {
            println!("❌ Too many target photos: {}", request.photos.len());
            return Err(anyhow::anyhow!("Maximum 15 target photos allowed"));
        }
        
        let mut uploaded_targets = Vec::new();

        // Dedup set — tracks (other_evidence_id, link_type) pairs already
        // linked+notified this batch. Stops 8+ duplicate notifications when
        // multiple target frames all match the same existing case.
        let mut seen_links: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();

        // Maps target.id -> local temp file path written from in-memory image bytes.
        // Passed directly to update_pickle.py so it reads from disk, not Storj.
        let mut local_tmp_paths: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        // Process each target photo
        for (index, photo_data) in request.photos.iter().enumerate() {
            let target_number = (index + 1) as i32;
            println!("🎯 Processing target photo {}: {}", target_number, photo_data.filename);
            
            // Decode base64 image
            let image_data = BASE64_STANDARD.decode(&photo_data.data_base64)
                .context("Failed to decode base64 image")?;
            
            // Validate image size (5MB max)
            let max_size = 5 * 1024 * 1024; // 5MB
            if image_data.len() > max_size {
                println!("❌ Target photo too large: {} bytes", image_data.len());
                return Err(anyhow::anyhow!("Target photo too large (max 5MB): {}", photo_data.filename));
            }
            
            // Determine country from evidence location
            let country = if !evidence.location.county.is_empty() {
                "Kenya".to_string()
            } else {
                "Unknown".to_string()
            };
            
            // Upload to Storj TARGET directory with country-based structure
            let filename = format!("target_{}_{}_{}", 
                request.evidence_id, 
                target_number, 
                photo_data.filename);
            
            println!("📤 Uploading target photo to Storj: {}/target/", country);
            
            let upload_result = self.storj_service.upload_bytes_with_public_access_target(
                &image_data,
                &filename,
                &photo_data.mime_type,
                &country, // Pass country for directory structure
            ).await.context(format!("Failed to upload target photo to Storj: {}", filename))?;
            
            println!("✅ Target photo uploaded: {}", upload_result.public_url);

            // Write image bytes to local temp file for pickle encoding.
            // Python reads this directly — no Storj round-trip needed.
            {
                let tmp_path = format!("/tmp/pickle_target_{}.jpg", Uuid::new_v4());
                if let Err(e) = tokio::fs::write(&tmp_path, &image_data).await {
                    println!("⚠️  [PICKLE] Could not write temp file {}: {}", tmp_path, e);
                } else {
                    println!("🥒 [PICKLE] Temp file ready: {}", tmp_path);
                    // Store keyed by a placeholder — will be updated after target.id is created
                    local_tmp_paths.insert(target_number.to_string(), tmp_path);
                }
            }

            // ── Layer 0: SHA-256 exact duplicate detection ────────────────────────
            let mut hasher = Sha256::new();
            hasher.update(&image_data);
            let file_hash = hex::encode(hasher.finalize());

            println!("🔍 [Layer 0] Checking exact hash: {}...", &file_hash[..8]);

            match self.database.check_target_hash_exists(&file_hash).await {
                Ok(existing_matches) if !existing_matches.is_empty() => {
                    println!("🎯 [Layer 0] EXACT MATCH in {} case(s)", existing_matches.len());
                    for (existing_evidence_id, existing_filename, existing_category, existing_confidence)
                        in existing_matches.iter()
                    {
                        if existing_evidence_id == &request.evidence_id { continue; }
                        let link_reason = format!(
                            "Identical {} detected in both cases (exact hash match). File: {}",
                            photo_data.category, existing_filename
                        );
                        link_and_notify(
                            &self.database,
                            &request.evidence_id,
                            existing_evidence_id,
                            "matched_target",
                            &link_reason,
                            &file_hash,
                            *existing_confidence,
                            user_id,
                        ).await;
                    }
                }
                Ok(_) => println!("✅ [Layer 0] No exact hash match"),
                Err(e) => println!("⚠️  [Layer 0] Hash check error: {} — continuing", e),
            }

            // ── Layer 1: Face detection + descriptor matching ─────────────────────
            println!("🧬 [Layer 1] Sending image to face sidecar...");

            let face_analysis = self.face_client.analyze(&image_data, &photo_data.mime_type).await;

            // pHash is always returned by sidecar regardless of whether a face was found
            let phash_value: Option<String> = face_analysis.as_ref()
                .and_then(|r| r.phash.clone());

            if let Some(ref analysis) = face_analysis {
                let faces = analysis.faces.as_deref().unwrap_or(&[]);

                if faces.is_empty() {
                    // ── No face detected — pHash fallback ────────────────────────────
                    println!("👤 [Layer 1] No face detected — running pHash fallback");

                    if let Some(ref ph) = phash_value {
                        let max_hamming: u32 = std::env::var("PHASH_MAX_DISTANCE")
                            .ok()
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(10);

                        match self.database.search_phash_matches(
                            ph,
                            max_hamming,
                            &request.evidence_id,
                        ).await {
                            Ok(matches) if !matches.is_empty() => {
                                println!(
                                    "🎯 [pHash] {} similar image(s) found (max_hamming={})",
                                    matches.len(), max_hamming
                                );
                                for m in &matches {
                                    println!(
                                        "   ↳ case {} | category={} | hamming={} | confidence={}%",
                                        m.evidence_number, m.category,
                                        m.hamming_distance, m.confidence_pct
                                    );
                                    let dedup_key = (m.evidence_id.clone(), "phash_match".to_string());
                                    if seen_links.contains(&dedup_key) { println!("   already notified this batch"); continue; }
                                    seen_links.insert(dedup_key);
                                    let link_reason = format!(
                                        "Visually similar {} detected via perceptual hash \
                                         (hamming distance={}, confidence={}%). \
                                         File: {}",
                                        m.category, m.hamming_distance,
                                        m.confidence_pct, m.filename
                                    );
                                    link_and_notify(
                                        &self.database,
                                        &request.evidence_id,
                                        &m.evidence_id,
                                        "phash_match",
                                        &link_reason,
                                        &file_hash,
                                        m.confidence_pct,
                                        user_id,
                                    ).await;
                                }
                            }
                            Ok(_) => println!("✅ [pHash] No similar images found — unique target"),
                            Err(e) => println!("⚠️  [pHash] Search error: {} — continuing", e),
                        }
                    } else {
                        println!("⚠️  [pHash] No pHash returned by sidecar — skipping fallback");
                    }
                } else {
                    println!("👤 [Layer 1] {} face(s) detected", faces.len());

                    let threshold = std::env::var("FACE_THRESHOLD")
                        .ok()
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.55);

                    for face in faces.iter() {
                        if face.descriptor.len() != 128 {
                            println!("⚠️  Skipping face {} — bad descriptor len", face.face_index);
                            continue;
                        }

                        // Search existing encodings for a match
                        match self.database.search_face_encodings(&face.descriptor, threshold).await {
                            Ok(matches) if !matches.is_empty() => {
                                println!("🎯 [Layer 1] FACE MATCH: {} similar face(s)", matches.len());
                                for m in &matches {
                                    if m.evidence_id == request.evidence_id { continue; }
                                    println!(
                                        "   ↳ case {} | dist={:.3} | confidence={}%",
                                        m.evidence_number, m.distance, m.confidence_score
                                    );
                                    let dedup_key = (m.evidence_id.clone(), "face_match".to_string());
                                    if seen_links.contains(&dedup_key) { println!("   already notified this batch"); continue; }
                                    seen_links.insert(dedup_key);
                                    let link_reason = format!(
                                        "Matching face detected (distance={:.3}, confidence={}%). \
                                         Face {} of {} in uploaded image.",
                                        m.distance, m.confidence_score,
                                        face.face_index + 1, faces.len()
                                    );
                                    link_and_notify(
                                        &self.database,
                                        &request.evidence_id,
                                        &m.evidence_id,
                                        "face_match",
                                        &link_reason,
                                        &file_hash,
                                        m.confidence_score,
                                        user_id,
                                    ).await;
                                }
                            }
                            Ok(_) => println!("✅ [Layer 1] No face match — new face"),
                            Err(e) => println!("⚠️  [Layer 1] Face search error: {} — continuing", e),
                        }
                    }
                }
            } else {
                println!("⚠️  [Layer 1] Face sidecar unavailable — skipping face match");
            }

            // ── Map category ──────────────────────────────────────────────────────
            let category = match photo_data.category.to_lowercase().as_str() {
                "person" => TargetCategory::Person,
                "vehicle" => TargetCategory::Vehicle,
                "object" => TargetCategory::Object,
                "location" => TargetCategory::Location,
                "other" => TargetCategory::Other,
                _ => {
                    println!("⚠️ Unknown category: {}, defaulting to 'other'", photo_data.category);
                    TargetCategory::Other
                }
            };

            // ── Create and save the primary target record ─────────────────────────
            let target = TargetPhoto {
                id: format!("target_{}", Uuid::new_v4()),
                evidence_id: request.evidence_id.clone(),
                target_number,
                filename: photo_data.filename.clone(),
                mime_type: photo_data.mime_type.clone(),
                file_size: image_data.len() as u64,
                description: photo_data.description.clone(),
                category,
                confidence_score: photo_data.confidence_score,
                storj_url: upload_result.public_url.clone(),
                storj_key: upload_result.key,
                hash: file_hash.clone(),
                phash: phash_value.clone(),
                auto_generated: false,  // uploader explicitly selected this target
                created_at: Utc::now(),
                created_by: user_id.to_string(),
            };

            match self.database.create_target(&target).await {
                Ok(_) => {},
                Err(e) => {
                    let msg = e.to_string().to_lowercase();
                    if msg.contains("unique") || msg.contains("duplicate") || msg.contains("already exists") {
                        println!("⚠️  Target {} already exists — skipping (retry)", target_number);
                        uploaded_targets.push(target);
                        continue;
                    }
                    return Err(e).context(format!("Failed to save target photo to database: {}", filename));
                }
            }

            // ── Store face encoding(s) now that we have the target.id ─────────────
            if let Some(ref analysis) = face_analysis {
                let faces = analysis.faces.as_deref().unwrap_or(&[]);

                if faces.is_empty() {
                    // ── No face — store pHash-only encoding row ───────────────────
                    // descriptor = 128 zeroes (sentinel — never used for face search)
                    // This row exists purely so pHash is anchored to target.id for
                    // future reverse lookups. detection_score = 0.0 marks it as non-face.
                    if phash_value.is_some() {
                        let zero_descriptor = vec![0.0f32; 128];
                        if let Err(e) = self.database.insert_face_encoding(
                            &target.id,
                            &request.evidence_id,
                            0,
                            &zero_descriptor,
                            0.0,                      // detection_score = 0 = no face
                            phash_value.as_deref(),
                            false,
                        ).await {
                            println!("⚠️  Failed to store pHash-only encoding: {}", e);
                        } else {
                            println!("🔷 [pHash] Stored pHash-only encoding for non-face target");
                        }
                    }
                } else {
                for face in faces.iter() {
                    if face.descriptor.len() != 128 { continue; }

                    // face_index 0 = largest/primary face (auto_generated = false)
                    // face_index 1+ = additional faces found in same image (auto_generated = true)
                    let is_extra = face.face_index > 0;

                    if let Err(e) = self.database.insert_face_encoding(
                        &target.id,
                        &request.evidence_id,
                        face.face_index as i32,
                        &face.descriptor,
                        face.detection_score,
                        phash_value.as_deref(),
                        is_extra,
                    ).await {
                        println!("⚠️  Failed to store face encoding: {}", e);
                    }

                    // ── Auto-create target profiles for extra faces ───────────────
                    // Face index 0 is the primary target the user selected.
                    // Additional faces get their own auto-generated target profiles.
                    if is_extra {
                        // Auto-target numbers must start AFTER all user-submitted photos
                        // to avoid colliding with upcoming targets in this batch.
                        // e.g. 3 user photos (1,2,3) → auto targets start at 4+
                        let next_number = request.photos.len() as i32
                            + uploaded_targets.len() as i32
                            + face.face_index as i32;
                        let auto_target = TargetPhoto {
                            id: format!("target_{}", Uuid::new_v4()),
                            evidence_id: request.evidence_id.clone(),
                            target_number: next_number,
                            filename: format!("auto_face_{}_{}", face.face_index, photo_data.filename),
                            mime_type: photo_data.mime_type.clone(),
                            file_size: image_data.len() as u64,
                            description: Some(format!(
                                "Auto-detected face #{} from image {} (detection confidence: {:.0}%)",
                                face.face_index + 1,
                                photo_data.filename,
                                face.detection_score * 100.0
                            )),
                            category: TargetCategory::Person,
                            confidence_score: (face.detection_score * 100.0) as i32,
                            storj_url: upload_result.public_url.clone(), // same image, different face
                            storj_key: format!("auto_{}", upload_result.public_url.clone()),
                            hash: file_hash.clone(),
                            phash: phash_value.clone(),
                            auto_generated: true,
                            created_at: Utc::now(),
                            created_by: user_id.to_string(),
                        };

                        match self.database.create_target(&auto_target).await {
                            Ok(_) => {
                                println!(
                                    "👤 Auto-created target profile for face #{} (target_number={})",
                                    face.face_index + 1, next_number
                                );

                                // Store the encoding linked to the auto-target id
                                let _ = self.database.insert_face_encoding(
                                    &auto_target.id,
                                    &request.evidence_id,
                                    face.face_index as i32,
                                    &face.descriptor,
                                    face.detection_score,
                                    phash_value.as_deref(),
                                    true,
                                ).await;

                                // Notify uploader about auto-detected faces
                                let _ = self.database.create_notification(
                                    user_id,
                                    "auto_face_detected",
                                    "👤 Additional Face Detected",
                                    &format!(
                                        "An additional face was automatically detected in your upload \
                                         for case {}. A target profile has been created — please review \
                                         and confirm or remove it.",
                                        request.evidence_id
                                    ),
                                    Some(&request.evidence_id),
                                    None,
                                    Some(&file_hash),
                                ).await;
                            }
                            Err(e) => println!("⚠️  Failed to create auto-target: {}", e),
                        }
                    }
                }
                } // end else faces not empty
            }

            // Remap temp file slot to the real target.id so pickle block can find it
            if let Some(tmp_path) = local_tmp_paths.remove(&format!("slot_{}", target_number)) {
                local_tmp_paths.insert(target.id.clone(), tmp_path);
            }
            uploaded_targets.push(target);
            println!("✅ Target photo {} saved successfully", target_number);
        }
        
        // ─── 🥒 Update encodings.pickle (non-blocking background task) ─────────
        {
            // Build payload using local temp file paths — Python reads directly from disk,
            // skipping any Storj network round-trip entirely.
            let pickle_targets: Vec<serde_json::Value> = uploaded_targets
                .iter()
                .filter(|t| !t.auto_generated)
                .filter_map(|t| {
                    local_tmp_paths.get(&t.target_number.to_string()).map(|path| serde_json::json!({
                        "target_id":   t.id,
                        "evidence_id": t.evidence_id,
                        "name":        evidence.evidence_number,
                        "local_path":  path,
                    }))
                })
                .collect();

            if !pickle_targets.is_empty() {
                let storj        = self.storj_service.clone();
                let payload_str  = serde_json::to_string(&pickle_targets)
                    .unwrap_or_else(|_| "[]".to_string());
                let n = pickle_targets.len();

                tokio::spawn(async move {
                    println!("🥒 [PICKLE] Spawning encoding update for {} target(s)", n);
                    // No existing_url needed — Python reads ./encodings/encodings.pickle from disk
                    let result = tokio::process::Command::new("python3")
                        .arg("update_pickle.py")
                        .arg(&payload_str)
                        .output()
                        .await;
                    match result {
                        Ok(out) => {
                            let logs = String::from_utf8_lossy(&out.stderr);
                            for line in logs.lines() { println!("{}", line); }
                            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                            // Scan all stdout lines for OK: — DeepFace may print noise before it
                            let ok_line = stdout.lines()
                                .find(|l| l.starts_with("OK:"))
                                .map(|l| l.trim_start_matches("OK:").to_string());
                            if let Some(pickle_path) = ok_line {
                                let pickle_path = pickle_path.trim();
                                println!("🥒 [PICKLE] Disk pickle updated → {}", pickle_path);
                                match tokio::fs::read(pickle_path).await {
                                    Ok(data) => {
                                        println!("🥒 [PICKLE] {} bytes — pushing to Storj...", data.len());
                                        match storj.upload_encoding_file(&data).await {
                                            Ok(url) => println!("✅ [PICKLE] Storj copy updated: {}", url),
                                            Err(e)  => println!("⚠️  [PICKLE] Storj push failed (disk still valid): {}", e),
                                        }
                                    }
                                    Err(e) => println!("⚠️  [PICKLE] Cannot read pickle: {}", e),
                                }
                            } else {
                                println!("⚠️  [PICKLE] {}", stdout);
                            }
                        }
                        Err(e) => println!("⚠️  [PICKLE] Failed to launch update_pickle.py: {}", e),
                    }
                });
            }
        }

        println!("🎯 UPLOAD_TARGETS: {} target photos uploaded for evidence {}", 
                uploaded_targets.len(), request.evidence_id);
        
        Ok(uploaded_targets)
    }


    /// Get target photos for evidence
    pub async fn get_evidence_targets(&self, evidence_id: &str) -> Result<Vec<TargetPhoto>> {
        println!("🎯 GET_EVIDENCE_TARGETS: Getting targets for evidence: {}", evidence_id);
        
        let targets = self.database.get_targets_for_evidence(evidence_id).await?;
        
        println!("🎯 Found {} target photos for evidence {}", targets.len(), evidence_id);
        
        Ok(targets)
    }

    // === NEW METHODS NEEDED ===
    
    pub async fn complete_evidence(
        &self,
        evidence_id: &str,
        user_id: &str,
        notes: Option<String>,
    ) -> Result<Evidence> {
        let mut evidence = match self.database.get_evidence(evidence_id, false).await? {
            Some(e) => e,
            None => return Err(anyhow::anyhow!("Evidence not found")),
        };

        // Verify ownership
        if evidence.uploader_id != user_id {
            return Err(anyhow::anyhow!("You don't own this evidence"));
        }

        // Update status
        evidence.status = EvidenceStatus::Submitted;
        evidence.updated_at = Utc::now();

        // Add to chain of custody
        evidence.chain_of_custody.push(CustodyRecord {
            action: "completed".to_string(),
            timestamp: Utc::now(),
            user_id: user_id.to_string(),
            details: format!("Evidence marked as complete: {:?}", notes),
        });

        self.database.update_evidence(&evidence).await?;
        Ok(evidence)
    }

    pub async fn create_evidence_with_media(
        &self,
        form: EvidenceForm,
        uploader: &SessionUser,
        auth_service: &AuthService,
    ) -> Result<Evidence> {
        // Call the main create_evidence with empty files
        self.create_evidence(form, uploader, Vec::new(), false, auth_service).await
    }

    pub async fn upload_evidence_media(
        &self,
        evidence_id: &str,
        media_data: Vec<u8>,
        media_type: &str,
        _user_id: &str, // Prefix with underscore to suppress warning
    ) -> Result<String> {
        let filename = format!("evidence_{}_{}.dat", evidence_id, Uuid::new_v4());
        
        let upload_result = self.storj_service.upload_bytes_with_public_access(
            &media_data,
            &filename,
            media_type,
        ).await.context("Failed to upload media")?;
        
        Ok(upload_result.public_url)
    }

    pub async fn submit_evidence(&self, evidence_id: &str, user_id: &str) -> Result<Evidence> {
        let mut evidence = match self.database.get_evidence(evidence_id, false).await? {
            Some(e) => e,
            None => return Err(anyhow::anyhow!("Evidence not found")),
        };

        if evidence.uploader_id != user_id {
            return Err(anyhow::anyhow!("You don't own this evidence"));
        }

        evidence.status = EvidenceStatus::Submitted;
        evidence.updated_at = Utc::now();

        self.database.update_evidence(&evidence).await?;
        Ok(evidence)
    }

    // In evidence_service.rs, fix the report_evidence_to_police method:
    pub async fn report_evidence_to_police(
        &self,
        evidence_id: &str,
        user_id: &str,
        form: PoliceReportForm,  // Change from &String to PoliceReportForm
    ) -> Result<Evidence> {
        let mut evidence = match self.database.get_evidence(evidence_id, false).await? {
            Some(e) => e,
            None => return Err(anyhow::anyhow!("Evidence not found")),
        };

        if evidence.uploader_id != user_id {
            return Err(anyhow::anyhow!("You don't own this evidence"));
        }

        evidence.reported_to_police = true;
        evidence.police_station = Some(form.police_station);
        evidence.police_case_id = Some(form.report_number);
        evidence.status = EvidenceStatus::Reported;
        evidence.updated_at = Utc::now();

        self.database.update_evidence(&evidence).await?;
        Ok(evidence)
    }

    // In evidence_service.rs, add to the EvidenceService impl:
    pub async fn get_evidence_signatures(&self, evidence_id: &str) -> Result<Vec<EvidenceSignature>> {
        self.database.get_evidence_signatures(evidence_id).await
    }

    pub async fn delete_evidence(&self, evidence_id: &str, user_id: &str) -> Result<()> {
        let evidence = match self.database.get_evidence(evidence_id, false).await? {
            Some(e) => e,
            None => return Err(anyhow::anyhow!("Evidence not found")),
        };

        if evidence.uploader_id != user_id {
            return Err(anyhow::anyhow!("You don't own this evidence"));
        }

        // Note: We need to add delete_evidence method to Database
        // For now, we'll mark it as archived
        let mut updated = evidence;
        updated.status = EvidenceStatus::Archived;
        updated.updated_at = Utc::now();
        
        self.database.update_evidence(&updated).await?;
        Ok(())
    }

    // In your EvidenceService implementation
    pub async fn get_all_evidence_locations(&self) -> Result<Vec<EvidenceLocationData>> {
        self.database.get_all_evidence_locations().await
    }

    // In evidence_service.rs
    pub async fn get_statistics(&self) -> Result<StatisticsResponse> {
        self.database.get_all_statistics().await
    }


    pub async fn get_all_evidence(&self) -> Result<Vec<Evidence>> {
    println!("🔍 EVIDENCE_SERVICE: Getting all evidence");
    
    let start_time = std::time::Instant::now();
    let result = self.database.get_all_evidence().await;
    let elapsed = start_time.elapsed();
    
    match &result {
        Ok(evidence) => {
            println!("✅ EVIDENCE_SERVICE: Retrieved {} evidence records in {:?}", 
                    evidence.len(), elapsed);
        }
        Err(e) => {
            println!("❌ EVIDENCE_SERVICE: Error getting all evidence: {} (took {:?})", e, elapsed);
        }
    }
    
    result
    }



        // Add to evidence_service.rs
    pub async fn get_evidence_locations_with_filters(
        &self,
        filters: &EvidenceLocationFilters,
    ) -> Result<Vec<EvidenceLocationDataSup>> {
        self.database.get_evidence_locations_with_filters(filters).await
    }

    pub async fn get_evidence_map_statistics(
        &self,
        filters: &EvidenceLocationFilters,
    ) -> Result<EvidenceMapStatistics> {
        self.database.get_evidence_map_statistics(filters).await
    }

    // Add to evidence_service.rs
    pub async fn get_evidence_stats(&self, user_id: &str) -> Result<DashboardStats> {
        self.database.get_evidence_stats(user_id).await
    }

    pub async fn get_chart_data(&self, user_id: &str) -> Result<ChartData> {
    self.database.get_evidence_chart_data(user_id).await
}


// evidence_service.rs - Update these lines
pub async fn get_storage_stats(&self, user_id: &str) -> Result<MediaStorageStats> {
    self.database.get_storage_statistics(user_id).await
}

pub async fn get_storage_size_stats(&self, user_id: &str) -> Result<StorageSizeData> {
    self.database.get_storage_size_statistics(user_id).await
}

    
}

// ─────────────────────────────────────────────────────────────────────────────
// HELPER — link two cases + send both notifications
// Extracted so both Layer 0 and Layer 1 can call the same logic without
// duplicating the notification code.
// ─────────────────────────────────────────────────────────────────────────────
async fn link_and_notify(
    db:          &crate::Database,
    evidence_id: &str,
    other_id:    &str,
    link_type:   &str,
    link_reason: &str,
    hash:        &str,
    confidence:  i32,
    user_id:     &str,
) {
    match db.link_evidence_cases(
        evidence_id,
        other_id,
        link_type,
        link_reason,
        hash,
        confidence,
        Some(user_id),
    ).await {
        Ok(link_id) => {
            println!("🔗 Cases linked: {} ↔ {} (type: {})", evidence_id, other_id, link_type);

            // Fetch both evidence records for notification messages
            let cur = db.get_evidence(evidence_id, false).await.ok().flatten();
            let oth = db.get_evidence(other_id, false).await.ok().flatten();

            if let (Some(cur), Some(oth)) = (cur, oth) {
                // Notify the uploader of the NEW case
                let _ = db.create_notification(
                    &cur.uploader_id,
                    "target_match",
                    "🎯 Target Match Found!",
                    &format!(
                        "A target you uploaded (case {}) was matched in another report \
                         (case {}). This may indicate a repeat offender or related incident.",
                        cur.evidence_number, oth.evidence_number
                    ),
                    Some(evidence_id),
                    Some(&link_id),
                    Some(hash),
                ).await;

                // Notify the uploader of the EXISTING matched case
                let _ = db.create_notification(
                    &oth.uploader_id,
                    "target_match",
                    "🎯 Your Case Has a Match!",
                    &format!(
                        "A target from your report (case {}) was found in a new report \
                         (case {}). This strengthens your case with corroborating evidence.",
                        oth.evidence_number, cur.evidence_number
                    ),
                    Some(other_id),
                    Some(&link_id),
                    Some(hash),
                ).await;

                println!("📧 Notifications sent to both uploaders");
            }
        }
        Err(e) => println!("⚠️  Failed to link cases: {}", e),
    }
}