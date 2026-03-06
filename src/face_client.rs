// src/face_client.rs
// ─────────────────────────────────────────────────────────────────────────────
// Thin async HTTP client that talks to the face_service.js sidecar.
//
// The sidecar runs on http://127.0.0.1:3001 (configurable via FACE_SERVICE_URL).
// It accepts a base64-encoded image and returns face descriptors + pHash.
//
// Add to Cargo.toml:
//   reqwest  = { version = "0.11", features = ["json"] }
//   serde    = { version = "1",    features = ["derive"] }  ← already present
//
// Register in main.rs / lib.rs:
//   mod face_client;
//   pub use face_client::FaceClient;
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::{Result, anyhow};
use base64::prelude::*;
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// Request / Response types (mirror face_service.js shapes)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct AnalyzeRequest {
    image_base64: String,
    mime_type:    String,
}

/// One detected face within the image.
#[derive(Debug, Clone, Deserialize)]
pub struct FaceResult {
    pub face_index:      usize,
    /// 128-element descriptor vector
    pub descriptor:      Vec<f32>,
    /// face-api.js detection confidence (0.0 – 1.0)
    pub detection_score: f64,
    pub box_x:           Option<f64>,
    pub box_y:           Option<f64>,
    pub box_width:       Option<f64>,
    pub box_height:      Option<f64>,
    pub is_largest:      bool,
}

/// Full response from POST /analyze
#[derive(Debug, Clone, Deserialize)]
pub struct AnalyzeResponse {
    pub success:       bool,
    pub error:         Option<String>,
    /// All detected faces, sorted largest → smallest
    pub faces:         Option<Vec<FaceResult>>,
    /// 16-char hex perceptual hash of the full image
    pub phash:         Option<String>,
    pub face_count:    Option<usize>,
    pub processing_ms: Option<u64>,
    pub image_width:   Option<u32>,
    pub image_height:  Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
// FaceClient
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FaceClient {
    base_url: String,
    client:   reqwest::Client,
}

impl FaceClient {
    /// Create a new client.  `base_url` defaults to http://127.0.0.1:3001.
    pub fn new() -> Self {
        let base_url = std::env::var("FACE_SERVICE_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client for face service");

        println!("🧬 FaceClient initialised → {}", base_url);
        Self { base_url, client }
    }

    /// Check whether the sidecar is up and models are loaded.
    pub async fn health_check(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_)   => false,
        }
    }

    /// Analyse an image buffer.
    /// Returns `None` if the sidecar is unreachable (non-fatal — upload continues).
    pub async fn analyze(
        &self,
        image_bytes: &[u8],
        mime_type:   &str,
    ) -> Option<AnalyzeResponse> {
        let url     = format!("{}/analyze", self.base_url);
        let b64     = BASE64_STANDARD.encode(image_bytes);
        let payload = AnalyzeRequest {
            image_base64: b64,
            mime_type:    mime_type.to_string(),
        };

        match self.client.post(&url).json(&payload).send().await {
            Err(e) => {
                println!("⚠️  Face service unreachable: {} — skipping face match", e);
                None
            }
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<AnalyzeResponse>().await {
                    Ok(body) => {
                        if body.success {
                            Some(body)
                        } else {
                            println!(
                                "⚠️  Face service error (HTTP {}): {}",
                                status,
                                body.error.unwrap_or_default()
                            );
                            None
                        }
                    }
                    Err(e) => {
                        println!("⚠️  Face service bad response: {}", e);
                        None
                    }
                }
            }
        }
    }
}

impl Default for FaceClient { fn default() -> Self { Self::new() } }

// ─────────────────────────────────────────────────────────────────────────────
// INTEGRATION PATCH FOR evidence_service.rs
// ─────────────────────────────────────────────────────────────────────────────
//
// Below is the section that replaces the SHA-256 block in
// evidence_service.rs (lines ~1416 → ~1520).
//
// In your EvidenceService struct, add:
//   pub face_client: FaceClient,
//
// In EvidenceService::new(), add:
//   face_client: FaceClient::new(),
//
// Then replace the block starting at:
//   "// Generate hash for verification"
// through to the end of the match self.database.check_target_hash_exists block
// with the code in the PATCH section below.
//
// ─── PATCH START ─────────────────────────────────────────────────────────────
//
// // ── Layer 0: SHA-256 exact duplicate detection (keep as-is) ──────────────
// let mut hasher = Sha256::new();
// hasher.update(&image_data);
// let file_hash = hex::encode(hasher.finalize());
//
// println!("🔍 Checking for exact hash match: {}...", &file_hash[..8]);
//
// match self.database.check_target_hash_exists(&file_hash).await {
//     Ok(existing_matches) if !existing_matches.is_empty() => {
//         println!("🎯 EXACT MATCH: hash exists in {} case(s)", existing_matches.len());
//         for (existing_evidence_id, existing_filename, existing_category, existing_confidence)
//             in existing_matches.iter()
//         {
//             if existing_evidence_id == &request.evidence_id { continue; }
//             let link_reason = format!(
//                 "Identical {} detected in both cases (exact hash). File: {}",
//                 photo_data.category, existing_filename
//             );
//             link_and_notify(
//                 &self.database,
//                 &request.evidence_id,
//                 existing_evidence_id,
//                 "matched_target",
//                 &link_reason,
//                 &file_hash,
//                 *existing_confidence,
//                 user_id,
//             ).await;
//         }
//     }
//     Ok(_) => println!("✅ No exact hash match — proceeding to face analysis"),
//     Err(e) => println!("⚠️  Hash check error: {} — continuing", e),
// }
//
// // ── Layer 1: Face-api.js similarity matching ──────────────────────────────
// let face_result = self.face_client.analyze(&image_data, &photo_data.mime_type).await;
//
// // pHash — always stored whether or not a face was detected
// let phash_value: Option<String> = face_result.as_ref()
//     .and_then(|r| r.phash.clone());
//
// if let Some(ref analysis) = face_result {
//     let faces = analysis.faces.as_deref().unwrap_or(&[]);
//
//     if faces.is_empty() {
//         // ── No face detected → pHash fallback ────────────────────────────
//         println!("👤 No face detected — using pHash fallback");
//         if let Some(ref ph) = phash_value {
//             // TODO Phase 4: pHash similarity search against stored pHashes
//             println!("   pHash: {}", ph);
//         }
//     } else {
//         println!("👤 {} face(s) detected in image", faces.len());
//
//         for (face_idx, face) in faces.iter().enumerate() {
//             if face.descriptor.len() != 128 {
//                 println!("⚠️  Skipping face {} — descriptor len={}", face_idx, face.descriptor.len());
//                 continue;
//             }
//
//             // ── Search existing encodings for a match ─────────────────────
//             let threshold = std::env::var("FACE_THRESHOLD")
//                 .ok()
//                 .and_then(|s| s.parse::<f64>().ok())
//                 .unwrap_or(0.55);
//
//             match self.database.search_face_encodings(&face.descriptor, threshold).await {
//                 Ok(matches) if !matches.is_empty() => {
//                     println!("🎯 FACE MATCH: {} similar face(s) found", matches.len());
//                     for m in &matches {
//                         if m.evidence_id == request.evidence_id { continue; }
//                         println!(
//                             "   ↳ evidence {} | distance={:.3} | confidence={}%",
//                             m.evidence_number, m.distance, m.confidence_score
//                         );
//                         let link_reason = format!(
//                             "Matching face detected (distance={:.3}, confidence={}%). \
//                              Face {} of {} in uploaded image.",
//                             m.distance, m.confidence_score,
//                             face_idx + 1, faces.len()
//                         );
//                         link_and_notify(
//                             &self.database,
//                             &request.evidence_id,
//                             &m.evidence_id,
//                             "face_match",
//                             &link_reason,
//                             &file_hash,
//                             m.confidence_score,
//                             user_id,
//                         ).await;
//                     }
//                 }
//                 Ok(_) => println!("✅ No face match found — storing new encoding"),
//                 Err(e) => println!("⚠️  Face search error: {} — continuing", e),
//             }
//
//             // ── Store the descriptor (whether it matched or not) ──────────
//             // We need the target.id — this is set just below in the original
//             // code where `target` is constructed. Store after target is saved.
//             // See STORE ENCODING comment in the target creation block below.
//             let _ = self.database.insert_face_encoding(
//                 &target_id_placeholder,   // replace with target.id after creation
//                 &request.evidence_id,
//                 face_idx as i32,
//                 &face.descriptor,
//                 face.detection_score,
//                 phash_value.as_deref(),
//                 false,  // not auto-generated — uploader selected this target
//             ).await;
//
//             // ── Auto-create target profiles for extra faces ───────────────
//             // If this is not face_index 0 AND the face wasn't already a
//             // user-selected target, create an auto-generated target profile.
//             // Phase 2 foundation — Phase 3 will wire the auto-create fully.
//         }
//     }
// }
//
// ─── PATCH END ───────────────────────────────────────────────────────────────
//
// Helper function — add near bottom of evidence_service.rs:
//
// async fn link_and_notify(
//     db:              &Database,
//     evidence_id:     &str,
//     other_id:        &str,
//     link_type:       &str,
//     link_reason:     &str,
//     hash:            &str,
//     confidence:      i32,
//     user_id:         &str,
// ) {
//     match db.link_evidence_cases(
//         evidence_id, other_id, link_type, link_reason, hash, confidence, Some(user_id),
//     ).await {
//         Ok(link_id) => {
//             println!("🔗 Cases linked: {} ↔ {} ({})", evidence_id, other_id, link_type);
//             // notifications — same pattern as existing block in original code
//             if let (Ok(Some(cur)), Ok(Some(oth))) = (
//                 db.get_evidence(evidence_id, false).await,
//                 db.get_evidence(other_id, false).await,
//             ) {
//                 let _ = db.create_notification(
//                     &cur.uploader_id, "target_match",
//                     "🎯 Target Match Found!",
//                     &format!(
//                         "A target you uploaded (case {}) was matched in another report (case {}).",
//                         cur.evidence_number, oth.evidence_number
//                     ),
//                     Some(evidence_id), Some(&link_id), Some(hash),
//                 ).await;
//                 let _ = db.create_notification(
//                     &oth.uploader_id, "target_match",
//                     "🎯 Your Case Has a Match!",
//                     &format!(
//                         "A target from your report (case {}) was found in a new report (case {}).",
//                         oth.evidence_number, cur.evidence_number
//                     ),
//                     Some(other_id), Some(&link_id), Some(hash),
//                 ).await;
//             }
//         }
//         Err(e) => println!("⚠️  Failed to link cases: {}", e),
//     }
// }
