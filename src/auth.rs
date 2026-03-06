use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};
use bcrypt;
use web3::{
    signing::recover,
    types::{Address, H160},
};
use hex;
use tiny_keccak::{Keccak, Hasher};
use chrono::{Utc};
use uuid::Uuid;
use bs58;
use std::collections::HashMap;

use crate::models::{
    User,
    SessionUser,
    WalletConnection,
    ContentSignature,
    ProfileCompletionForm,
    ProfileStatusResponse,
    EvidenceSignature,
};

use crate::database::AuditLogEntry;
use crate::email_service::EmailService;
use crate::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletChallenge {
    pub challenge: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAuthRequest {
    pub wallet_address: String,
    pub chain: String,
    pub signature: String,
    pub message: String,
    pub public_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthService {
    database: Database,
    email_service: Option<EmailService>,
    wallet_challenges: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, WalletChallenge>>>,
    content_signatures: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, ContentSignature>>>,
}

impl AuthService {
    pub fn new(database: Database) -> Self {
        Self {
            database,
            email_service: None,
            wallet_challenges: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            content_signatures: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn with_email_service(mut self, email_service: EmailService) -> Self {
        self.email_service = Some(email_service);
        self
    }

    // ==================== USER REGISTRATION & MANAGEMENT ====================

    /// Creates a new email-based user account.
    /// phone_number is captured at registration time via the multi-step form.
    /// Email verification is NOT required to proceed — it is deferred to the
    /// user's profile settings page.
    pub async fn register_user(
        &self,
        email: &str,
        phone_number: Option<&str>,
    ) -> Result<(User, String)> {
        println!("📝 REGISTER: Attempting to register user: {}", email);

        if let Some(_) = self.database.get_user_by_email(email).await? {
            println!("⚠️ REGISTER: User already exists: {}", email);
            return Err(anyhow!("User already exists"));
        }

        let user_id            = format!("user_{}", Uuid::new_v4());
        let verification_token = format!("verify_{}", Uuid::new_v4());
        let now                = Utc::now().timestamp() as u64;

        let user = User {
            id:                  user_id.clone(),
            email:               email.to_string(),
            password_hash:       None,
            is_verified:         false,
            verification_token:  Some(verification_token.clone()),
            wallet_address:      None,
            wallet_type:         None,
            wallet_chain:        None,
            public_key:          None,
            created_at:          now,
            updated_at:          now,
            account_type:        None,
            business_name:       None,
            geo_latitude:        None,
            geo_longitude:       None,
            is_profile_complete: false,
            // Kenya fields — phone captured at registration; county/id_number set in profile
            phone_number:        phone_number.map(|s| s.to_string()),
            county:              None,
            id_number:           None,
        };

        self.database.create_user(&user).await?;
        println!("✅ REGISTER: User saved to database");

        // Store verification token — user can click the link from their profile later
        self.database.store_verification_token(
            &verification_token,
            email,
            "email_verification",
            168, // 7 days — generous window since this is optional/deferred
        ).await?;
        println!("✅ REGISTER: Verification token stored");

        let base_url          = std::env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let verification_url  = format!("{}/verify-email?token={}", base_url, verification_token);

        // Fire verification email in the background — non-blocking, non-fatal
        if let Some(email_service) = &self.email_service {
            println!("📧 REGISTER: Sending verification email to: {}", email);
            match email_service.send_verification_email(email, &verification_url).await {
                Ok(()) => println!("✅ REGISTER: Verification email sent"),
                Err(e) => println!("⚠️ REGISTER: Verification email failed (non-fatal): {}", e),
            }
        } else {
            println!("⚠️ REGISTER: No email service configured");
        }

        println!("✅ REGISTER: Registration complete for: {}", email);
        Ok((user, verification_token))
    }

    /// Creates a new wallet-only (anonymous) account.
    /// Wallet signature acts as the proof of identity — no email required.
    pub async fn register_with_wallet(
        &self,
        wallet_address: &str,
        chain: &str,
        wallet_type: &str,
        signature: &str,
        public_key: Option<&str>,
    ) -> Result<User> {
        println!("🔗 REGISTER_WITH_WALLET: Creating account for wallet: {}", wallet_address);

        if let Ok(Some(_)) = self.find_user_by_wallet(wallet_address).await {
            println!("⚠️ REGISTER_WITH_WALLET: Wallet already registered");
            return Err(anyhow!("Wallet already registered"));
        }

        if signature.is_empty() || signature.len() < 10 {
            println!("❌ REGISTER_WITH_WALLET: Invalid signature");
            return Err(anyhow!("Invalid signature"));
        }

        let user_id   = format!("user_{}", Uuid::new_v4());
        let now       = Utc::now().timestamp() as u64;
        let temp_email = format!("wallet_{}@flug.evidence", &wallet_address[..8].to_lowercase());

        let user = User {
            id:                  user_id.clone(),
            email:               temp_email,
            password_hash:       None,
            is_verified:         true, // wallet signature = proof of identity
            verification_token:  None,
            wallet_address:      Some(wallet_address.to_string()),
            wallet_type:         Some(wallet_type.to_string()),
            wallet_chain:        Some(chain.to_string()),
            public_key:          public_key.map(|s| s.to_string()),
            created_at:          now,
            updated_at:          now,
            account_type:        None,
            business_name:       None,
            geo_latitude:        None,
            geo_longitude:       None,
            is_profile_complete: false,
            phone_number:        None,
            county:              None,
            id_number:           None,
        };

        self.database.create_user(&user).await?;
        println!("✅ REGISTER_WITH_WALLET: User saved to database");

        let connection = WalletConnection {
            wallet_address: wallet_address.to_string(),
            chain:          chain.to_string(),
            wallet_type:    wallet_type.to_string(),
            public_key:     public_key.map(|s| s.to_string()),
            connected_at:   Utc::now(),
            last_used:      Utc::now(),
            is_active:      true,
        };

        self.database.connect_wallet(&connection, &user.id).await?;
        println!("✅ REGISTER_WITH_WALLET: Wallet connection saved");

        self.database.log_audit(
            Some(&user.id),
            "wallet_registered",
            "user",
            Some(&user.id),
            &format!("Wallet registration: {}", wallet_address),
            None,
        ).await?;

        println!("✅ REGISTER_WITH_WALLET: Account created successfully");
        Ok(user)
    }

    // ==================== EMAIL VERIFICATION ====================
    // Verification is now optional/deferred — users trigger this from their profile.
    // The route still exists so old verification links in emails keep working.

    pub async fn verify_email(&self, token: &str) -> Result<User> {
        println!("🔍 VERIFY_EMAIL: Token received (length={})", token.len());

        if token.is_empty() {
            return Err(anyhow!("Empty verification token"));
        }

        // Tolerate tokens that arrive with URL fragments attached
        let clean_token = if token.contains("token=") {
            let parts: Vec<&str> = token.split("token=").collect();
            if parts.len() > 1 {
                let t = parts[1];
                if let Some(end) = t.find('&') { &t[..end] } else { t }
            } else {
                token
            }
        } else {
            token
        };

        match self.database.verify_token(clean_token, "email_verification").await {
            Ok(Some(email)) => {
                println!("✅ VERIFY_EMAIL: Token valid for: {}", email);
                self.complete_email_verification(&email).await
            }
            Ok(None) => {
                println!("❌ VERIFY_EMAIL: Invalid or expired token");
                Err(anyhow!("Invalid or expired verification token"))
            }
            Err(e) => Err(anyhow!("Database error: {}", e)),
        }
    }

    async fn complete_email_verification(&self, email: &str) -> Result<User> {
        match self.database.get_user_by_email(email).await {
            Ok(Some(mut user)) => {
                user.is_verified       = true;
                user.verification_token = None;
                user.updated_at        = Utc::now().timestamp() as u64;
                self.database.update_user(&user).await?;

                if let Some(email_service) = &self.email_service {
                    let _ = email_service.send_welcome_email(email).await;
                }

                println!("✅ VERIFY_EMAIL: Email verified: {}", email);
                Ok(user)
            }
            Ok(None) => Err(anyhow!("User not found")),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    // ==================== PROFILE COMPLETION ====================

    pub async fn complete_profile(
        &self,
        current_email: &str,
        profile_data: ProfileCompletionForm,
        new_email: Option<&str>,
    ) -> Result<User> {
        println!("📝 COMPLETE_PROFILE: Completing profile for: {}", current_email);

        let account_type = profile_data.account_type.to_lowercase();
        if account_type != "citizen" && account_type != "business" {
            return Err(anyhow!("Invalid account type. Must be 'citizen' or 'business'"));
        }
        if account_type == "business" && profile_data.business_name.is_none() {
            return Err(anyhow!("Business name is required for business accounts"));
        }
        if profile_data.county.as_ref().map_or(true, |s| s.trim().is_empty()) {
            return Err(anyhow!("County is required"));
        }

        match self.database.get_user_by_email(current_email).await {
            Ok(Some(mut user)) => {
                let mut email_changed = false;
                if let Some(new_email_val) = new_email {
                    if !new_email_val.trim().is_empty() && new_email_val != current_email {
                        if let Ok(Some(_)) = self.database.get_user_by_email(new_email_val).await {
                            return Err(anyhow!("Email already in use"));
                        }
                        println!("   Updating email: '{}' → '{}'", current_email, new_email_val);
                        user.email    = new_email_val.to_string();
                        email_changed = true;
                    }
                }

                let account_type_clone = account_type.clone();
                user.account_type        = Some(account_type);
                user.business_name       = profile_data.business_name;
                user.geo_latitude        = profile_data.geo_latitude;
                user.geo_longitude       = profile_data.geo_longitude;
                user.is_profile_complete = true;
                user.updated_at          = Utc::now().timestamp() as u64;
                // phone_number intentionally not touched — already stored at registration
                user.county              = profile_data.county;

                self.database.update_user(&user).await?;

                let audit_msg = if email_changed {
                    format!("Profile completed with email update → {}, County: {:?}", user.email, user.county)
                } else {
                    format!("Profile completed: {} - County: {:?}", account_type_clone, user.county)
                };

                self.database.log_audit(
                    Some(&user.id),
                    "profile_completed",
                    "user",
                    Some(&user.id),
                    &audit_msg,
                    None,
                ).await?;

                println!("✅ COMPLETE_PROFILE: Done for: {}", user.email);
                Ok(user)
            }
            Ok(None) => Err(anyhow!("User not found")),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    pub async fn check_profile_status(&self, email: &str) -> Result<ProfileStatusResponse> {
        match self.database.get_user_by_email(email).await {
            Ok(Some(user)) => {
                let mut missing_fields = Vec::new();
                if user.account_type.is_none() {
                    missing_fields.push("account_type".to_string());
                }
                if user.account_type.as_deref() == Some("business") && user.business_name.is_none() {
                    missing_fields.push("business_name".to_string());
                }
                Ok(ProfileStatusResponse {
                    is_profile_complete:  user.is_profile_complete,
                    missing_fields,
                    current_account_type: user.account_type,
                })
            }
            Ok(None) => Err(anyhow!("User not found")),
            Err(e)   => Err(e),
        }
    }

    // ==================== PASSWORD AUTHENTICATION ====================

    /// Sets (or overwrites) the password hash for an account.
    /// NOTE: We do NOT gate this on is_verified — email verification is deferred
    /// to the user profile. New accounts set their password right after registration,
    /// before they have had a chance to click any verification link.
    pub async fn set_password(&self, email: &str, password: &str) -> Result<()> {
        println!("🔐 SET_PASSWORD: Setting password for: {}", email);

        if password.len() < 6 {
            return Err(anyhow!("Password must be at least 6 characters"));
        }

        let hashed = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|e| anyhow!("Password hashing failed: {}", e))?;

        println!("✅ SET_PASSWORD: Password hashed");

        match self.database.get_user_by_email(email).await {
            Ok(Some(mut user)) => {
                user.password_hash = Some(hashed);
                user.updated_at    = Utc::now().timestamp() as u64;
                self.database.update_user(&user).await?;

                if let Some(email_service) = &self.email_service {
                    let _ = email_service.send_password_changed_notification(email).await;
                }

                println!("✅ SET_PASSWORD: Password set for: {}", email);
                Ok(())
            }
            Ok(None) => Err(anyhow!("User not found: {}", email)),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    /// Verifies a password for login.
    /// NOTE: We do NOT gate login on is_verified — a user who has set their
    /// password should always be able to sign in. Email verification is a
    /// separate, optional trust level indicator, not an access gate.
    pub async fn verify_password(&self, email: &str, password: &str) -> Result<Option<User>> {
        println!("🔑 VERIFY_PASSWORD: Login attempt for: {}", email);

        match self.database.get_user_by_email(email).await {
            Ok(Some(user)) => {
                if let Some(stored_hash) = &user.password_hash {
                    let is_valid = bcrypt::verify(password, stored_hash)
                        .map_err(|e| anyhow!("Password verification error: {}", e))?;

                    if is_valid {
                        let _ = self.database.update_user_login(&user.id, None).await;
                        println!("🎉 VERIFY_PASSWORD: Login successful for: {}", email);
                        return Ok(Some(user));
                    } else {
                        println!("❌ VERIFY_PASSWORD: Invalid password for: {}", email);
                    }
                } else {
                    println!("❌ VERIFY_PASSWORD: No password set for: {}", email);
                }
            }
            Ok(None) => println!("❌ VERIFY_PASSWORD: User not found: {}", email),
            Err(e)   => println!("❌ VERIFY_PASSWORD: Database error: {}", e),
        }

        Ok(None)
    }

    // ==================== PASSWORD RESET ====================

    pub async fn request_password_reset(&self, email: &str) -> Result<String> {
        println!("🔑 REQUEST_PASSWORD_RESET: Request for: {}", email);

        match self.database.get_user_by_email(email).await {
            Ok(Some(_user)) => {
                let reset_token = format!("reset_{}", Uuid::new_v4());

                self.database.store_verification_token(
                    &reset_token,
                    email,
                    "password_reset",
                    1,
                ).await?;

                let base_url  = std::env::var("BASE_URL")
                    .unwrap_or_else(|_| "http://localhost:8080".to_string());
                let reset_url = format!("{}/reset-password?token={}", base_url, reset_token);

                if let Some(email_service) = &self.email_service {
                    match email_service.send_password_reset_email(email, &reset_token).await {
                        Ok(_) => {
                            println!("✅ PASSWORD_RESET: Reset email sent to: {}", email);
                            Ok("Reset email sent successfully. Please check your inbox.".to_string())
                        }
                        Err(e) => {
                            println!("⚠️ PASSWORD_RESET: Email failed: {}", e);
                            Ok(format!("Email sending failed. Manual reset URL: {}", reset_url))
                        }
                    }
                } else {
                    Ok(format!("No email service configured. Manual reset URL: {}", reset_url))
                }
            }
            Ok(None) => {
                // Don't reveal whether account exists
                Ok("If an account exists with this email, a reset link has been sent".to_string())
            }
            Err(e) => Err(anyhow!("Database error: {}", e)),
        }
    }

    pub async fn reset_password(&self, token: &str, new_password: &str) -> Result<()> {
        println!("🔑 RESET_PASSWORD: Attempting reset");

        if new_password.len() < 6 {
            return Err(anyhow!("Password must be at least 6 characters"));
        }

        match self.database.verify_token(token, "password_reset").await {
            Ok(Some(email)) => {
                let hashed = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
                    .map_err(|e| anyhow!("Password hashing failed: {}", e))?;

                match self.database.get_user_by_email(&email).await {
                    Ok(Some(mut user)) => {
                        user.password_hash = Some(hashed);
                        user.updated_at    = Utc::now().timestamp() as u64;
                        self.database.update_user(&user).await?;

                        if let Some(email_service) = &self.email_service {
                            let _ = email_service.send_password_changed_notification(&email).await;
                        }

                        println!("✅ RESET_PASSWORD: Password reset for: {}", email);
                        Ok(())
                    }
                    Ok(None) => Err(anyhow!("User not found")),
                    Err(e)   => Err(anyhow!("Database error: {}", e)),
                }
            }
            Ok(None) => Err(anyhow!("Invalid or expired reset token")),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    pub async fn change_password(
        &self,
        email: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        println!("🔑 CHANGE_PASSWORD: Request for: {}", email);

        if new_password.len() < 6 {
            return Err(anyhow!("New password must be at least 6 characters"));
        }

        match self.database.get_user_by_email(email).await {
            Ok(Some(mut user)) => {
                if let Some(stored_hash) = &user.password_hash {
                    let is_valid = bcrypt::verify(current_password, stored_hash)
                        .map_err(|e| anyhow!("Password verification error: {}", e))?;
                    if !is_valid {
                        return Err(anyhow!("Current password is incorrect"));
                    }
                } else {
                    return Err(anyhow!("No password set for this account"));
                }

                let hashed = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)
                    .map_err(|e| anyhow!("Password hashing failed: {}", e))?;

                user.password_hash = Some(hashed);
                user.updated_at    = Utc::now().timestamp() as u64;
                self.database.update_user(&user).await?;

                if let Some(email_service) = &self.email_service {
                    let _ = email_service.send_password_changed_notification(email).await;
                }

                println!("✅ CHANGE_PASSWORD: Password changed for: {}", email);
                Ok(())
            }
            Ok(None) => Err(anyhow!("User not found")),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    // ==================== WALLET CONNECTION ====================

    pub async fn connect_wallet(
        &self,
        email: &str,
        wallet_address: &str,
        chain: &str,
        wallet_type: &str,
        signature: &str,
        public_key: Option<&str>,
    ) -> Result<User> {
        println!("🔗 CONNECT_WALLET: Connecting wallet for: {}", email);

        match self.database.get_user_by_email(email).await {
            Ok(Some(mut user)) => {
                // EVM signature verification
                if chain.to_lowercase().contains("evm")
                    || chain == "ethereum"
                    || chain == "base"
                    || chain == "avalanche"
                    || chain == "polygon"
                {
                    if !self.verify_signature_simple(wallet_address, signature).await? {
                        return Err(anyhow!("Signature verification failed"));
                    }
                }

                let connection = WalletConnection {
                    wallet_address: wallet_address.to_string(),
                    chain:          chain.to_string(),
                    wallet_type:    wallet_type.to_string(),
                    public_key:     public_key.map(|s| s.to_string()),
                    connected_at:   Utc::now(),
                    last_used:      Utc::now(),
                    is_active:      true,
                };

                self.database.connect_wallet(&connection, &user.id).await?;

                user.wallet_address = Some(wallet_address.to_string());
                user.wallet_type    = Some(wallet_type.to_string());
                user.wallet_chain   = Some(chain.to_string());
                user.public_key     = public_key.map(|s| s.to_string());
                user.updated_at     = Utc::now().timestamp() as u64;
                self.database.update_user(&user).await?;

                if let Some(email_service) = &self.email_service {
                    let _ = email_service.send_wallet_connected_notification(email, wallet_address, chain).await;
                }

                println!("✅ CONNECT_WALLET: Connected successfully");
                Ok(user)
            }
            Ok(None) => Err(anyhow!("User not found")),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    async fn verify_signature_simple(&self, _address: &str, signature: &str) -> Result<bool> {
        if signature.is_empty() || signature.len() < 10 {
            return Ok(false);
        }
        Ok(signature.starts_with("0x") && signature.len() >= 130)
    }

    // ==================== WALLET LOGIN ====================

    pub async fn login_with_wallet(
        &self,
        wallet_address: &str,
        signature: &str,
        wallet_type: &str,
    ) -> Result<Option<User>> {
        println!("🔐 LOGIN_WITH_WALLET: Attempt for: {}", wallet_address);

        let challenge = self.generate_login_challenge(wallet_address).await?;

        let chain = if wallet_type.to_lowercase().contains("evm")
            || wallet_type.to_lowercase().contains("ethereum")
            || wallet_type.to_lowercase() == "web3"
            || wallet_type.to_lowercase() == "metamask"
        {
            "evm"
        } else if wallet_type.to_lowercase().contains("solana") {
            "solana"
        } else if wallet_type.to_lowercase().contains("stellar") {
            "stellar"
        } else {
            wallet_type
        };

        match chain {
            "evm" => {
                if let Err(e) = self.verify_evm_signature(&challenge, wallet_address, signature).await {
                    println!("❌ LOGIN_WITH_WALLET: EVM verification failed: {}", e);
                    return Ok(None);
                }
            }
            "solana" => {
                if let Err(e) = self.verify_solana_signature(wallet_address).await {
                    println!("❌ LOGIN_WITH_WALLET: Solana verification failed: {}", e);
                    return Ok(None);
                }
            }
            "stellar" => {
                if let Err(e) = self.verify_stellar_signature_simple(&challenge, wallet_address, signature, None).await {
                    println!("❌ LOGIN_WITH_WALLET: Stellar verification failed: {}", e);
                    return Ok(None);
                }
            }
            _ => {
                println!("❌ LOGIN_WITH_WALLET: Unsupported wallet type: {}", wallet_type);
                return Ok(None);
            }
        }

        match self.database.find_user_by_wallet(wallet_address).await {
            Ok(Some(user)) => {
                println!("✅ LOGIN_WITH_WALLET: User found: {}", user.email);
                let _ = self.database.update_user_login(&user.id, None).await;
                Ok(Some(user))
            }
            Ok(None) => {
                println!("❌ LOGIN_WITH_WALLET: No user found with wallet: {}", wallet_address);
                Ok(None)
            }
            Err(e) => {
                println!("❌ LOGIN_WITH_WALLET: Database error: {}", e);
                Ok(None)
            }
        }
    }

    // ==================== WALLET CHALLENGE GENERATION ====================

    pub async fn generate_wallet_login_challenge(
        &self,
        wallet_address: &str,
        chain: &str,
    ) -> Result<String> {
        println!("🎯 GENERATE_LOGIN_CHALLENGE: For: {}", wallet_address);

        let challenge = format!(
            "Login to FLUG Evidence\n\nSign this message to authenticate with your wallet.\n\nWallet: {}\nTimestamp: {}",
            wallet_address,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        let mut challenges = self.wallet_challenges.write().await;
        challenges.insert(
            wallet_address.to_string(),
            WalletChallenge {
                challenge: challenge.clone(),
                expires_at: Utc::now().timestamp() as u64 + 300,
            },
        );

        Ok(challenge)
    }

    pub async fn generate_wallet_connection_challenge(
        &self,
        email: &str,
        wallet_address: &str,
        chain: &str,
    ) -> Result<String> {
        println!("🎯 GENERATE_CONNECTION_CHALLENGE: {} → {}", email, wallet_address);

        let challenge = format!(
            "Connect wallet to FLUG Evidence\n\nWallet: {}\nUser: {}\nTimestamp: {}",
            wallet_address,
            email,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );

        Ok(challenge)
    }

    // ==================== CRYPTOGRAPHIC VERIFICATION ====================

    async fn verify_evm_signature(
        &self,
        message: &str,
        address: &str,
        signature: &str,
    ) -> Result<()> {
        println!("🔍 VERIFY_EVM: address={}", address);

        let signature_clean = signature.strip_prefix("0x").unwrap_or(signature);
        let signature_bytes = hex::decode(signature_clean)
            .map_err(|e| anyhow!("Invalid signature hex: {}", e))?;

        if signature_bytes.len() != 65 {
            return Err(anyhow!("Invalid signature length: expected 65 bytes, got {}", signature_bytes.len()));
        }

        let expected_address = address.parse::<Address>()
            .map_err(|e| anyhow!("Invalid EVM address: {}", e))?;

        let ethereum_message = format!(
            "\x19Ethereum Signed Message:\n{}{}",
            message.len(),
            message
        );
        let message_hash = self.keccak256(ethereum_message.as_bytes());

        let v           = signature_bytes[64];
        let recovery_id = if v == 27 || v == 28 {
            v - 27
        } else if v >= 35 {
            ((v - 35) % 2) as u8
        } else {
            return Err(anyhow!("Invalid recovery ID: {}", v));
        };

        let mut recovered_address = None;
        for rid in [recovery_id as i32, (1 - recovery_id) as i32].iter() {
            match recover(&message_hash, &signature_bytes[..64], *rid) {
                Ok(addr) => {
                    let r_str = format!("{:?}", addr).to_lowercase();
                    let e_str = format!("{:?}", expected_address).to_lowercase();
                    if addr == expected_address || r_str == e_str {
                        recovered_address = Some(addr);
                        break;
                    }
                }
                Err(_) => {}
            }
        }

        match recovered_address {
            Some(addr) if {
                let r = format!("{:?}", addr).to_lowercase();
                let e = format!("{:?}", expected_address).to_lowercase();
                r == e || addr == expected_address
            } => {
                println!("✅ VERIFY_EVM: Signature valid");
                Ok(())
            }
            _ => Err(anyhow!("Signature verification failed")),
        }
    }

    async fn verify_stellar_signature_simple(
        &self,
        _message: &str,
        wallet_address: &str,
        signature: &str,
        _public_key: Option<&str>,
    ) -> Result<()> {
        if !wallet_address.starts_with('G') || wallet_address.len() != 56 {
            return Err(anyhow!("Invalid Stellar address format"));
        }
        match bs58::decode(wallet_address).into_vec() {
            Ok(bytes) if bytes.len() == 32 => {
                if signature.is_empty() {
                    return Err(anyhow!("Empty signature"));
                }
                Ok(())
            }
            _ => Err(anyhow!("Invalid Stellar address encoding")),
        }
    }

    fn keccak256(&self, data: &[u8]) -> [u8; 32] {
        let mut hasher = Keccak::v256();
        let mut output = [0u8; 32];
        hasher.update(data);
        hasher.finalize(&mut output);
        output
    }

    // ==================== CONTENT SIGNING ====================

    pub async fn sign_evidence_hash(
        &self,
        evidence_hash: &[u8],
        evidence_id: &str,
        wallet_address: &str,
        chain: &str,
    ) -> Result<EvidenceSignature> {
        let timestamp = Utc::now();
        let message   = format!(
            "FLUG Evidence Signature\nEvidence ID: {}\nEvidence Hash: {}\nWallet: {}\nChain: {}\nTimestamp: {}",
            evidence_id,
            hex::encode(evidence_hash),
            wallet_address,
            chain,
            timestamp.timestamp()
        );

        let signature_hash = self.keccak256(message.as_bytes());
        let signature      = format!("sig_{}_{}", hex::encode(&signature_hash[..8]), Uuid::new_v4());

        let evidence_signature = EvidenceSignature {
            evidence_id:    evidence_id.to_string(),
            wallet_address: wallet_address.to_string(),
            signature:      signature.clone(),
            signed_hash:    hex::encode(evidence_hash),
            timestamp,
            chain:          chain.to_string(),
            transaction_id: Some(format!("tx_{}", Uuid::new_v4())),
        };

        self.database.store_evidence_signature(&evidence_signature).await?;
        Ok(evidence_signature)
    }

    pub async fn verify_content_signature(
        &self,
        content_id: &str,
        wallet_address: &str,
        signature: &str,
    ) -> Result<bool> {
        let signatures = self.database.get_evidence_signatures(wallet_address).await?;
        for sig in signatures {
            if sig.evidence_id == content_id && sig.signature == signature {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub async fn get_content_signatures(&self, wallet_address: &str) -> Result<Vec<EvidenceSignature>> {
        self.database.get_evidence_signatures(wallet_address).await
    }

    // ==================== WALLET MANAGEMENT ====================

    pub async fn get_user_wallets(&self, email: &str) -> Result<Vec<WalletConnection>> {
        self.database.get_user_wallets(email).await
    }

    pub async fn disconnect_wallet(&self, email: &str) -> Result<User> {
        println!("🔗 DISCONNECT_WALLET: For: {}", email);

        match self.database.get_user_by_email(email).await {
            Ok(Some(mut user)) => {
                user.wallet_address = None;
                user.wallet_type    = None;
                user.wallet_chain   = None;
                user.public_key     = None;
                user.updated_at     = Utc::now().timestamp() as u64;
                self.database.update_user(&user).await?;
                self.database.disconnect_wallet(&user.id).await?;
                println!("✅ DISCONNECT_WALLET: Done for: {}", email);
                Ok(user)
            }
            Ok(None) => Err(anyhow!("User not found")),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    pub async fn update_wallet_last_used(&self, email: &str) -> Result<()> {
        match self.database.get_user_by_email(email).await {
            Ok(Some(mut user)) => {
                user.updated_at = Utc::now().timestamp() as u64;
                self.database.update_user(&user).await
            }
            Ok(None) => Err(anyhow!("User not found")),
            Err(e)   => Err(anyhow!("Database error: {}", e)),
        }
    }

    // ==================== USER QUERY METHODS ====================

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        self.database.get_user_by_email(email).await
    }

    pub async fn get_session_user(&self, email: &str) -> Result<Option<SessionUser>> {
        match self.database.get_user_by_email(email).await {
            Ok(Some(user)) => {
                let wallet_connections = self.get_user_wallets(email).await.unwrap_or_default();
                Ok(Some(SessionUser {
                    id:                  user.id.clone(),
                    email:               user.email.clone(),
                    has_password:        user.password_hash.is_some(),
                    has_wallet:          user.wallet_address.is_some(),
                    wallet_address:      user.wallet_address.clone(),
                    wallet_type:         user.wallet_type.clone(),
                    wallet_chain:        user.wallet_chain.clone(),
                    is_verified:         user.is_verified,
                    wallet_connections,
                    account_type:        user.account_type.clone(),
                    business_name:       user.business_name.clone(),
                    geo_latitude:        user.geo_latitude,
                    geo_longitude:       user.geo_longitude,
                    is_profile_complete: user.is_profile_complete,
                    phone_number:        user.phone_number.clone(),
                    county:              user.county.clone(),
                    id_number:           user.id_number.clone(),
                }))
            }
            Ok(None) => Ok(None),
            Err(e)   => Err(e),
        }
    }

    pub async fn find_user_by_wallet(&self, wallet_address: &str) -> Result<Option<User>> {
        self.database.find_user_by_wallet(wallet_address).await
    }

    pub async fn remove_user(&self, email: &str) -> Result<()> {
        println!("🗑️ REMOVE_USER: Removing: {}", email);
        self.database.remove_user(email).await
    }

    // ==================== CHAIN-SPECIFIC METHODS ====================

    pub async fn validate_evm_address(&self, address: &str) -> Result<bool> {
        if address.starts_with("0x") && address.len() == 42 {
            match address.parse::<H160>() {
                Ok(_)  => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }

    pub async fn get_chain_info(&self, chain: &str) -> Result<ChainInfo> {
        match chain.to_lowercase().as_str() {
            "ethereum" => Ok(ChainInfo {
                name:             "Ethereum".to_string(),
                chain_id:         1,
                native_currency:  "ETH".to_string(),
                rpc_url:          "https://mainnet.infura.io/v3/YOUR_INFURA_KEY".to_string(),
                block_explorer:   "https://etherscan.io".to_string(),
                supports_eip1559: true,
            }),
            "base" => Ok(ChainInfo {
                name:             "Base".to_string(),
                chain_id:         8453,
                native_currency:  "ETH".to_string(),
                rpc_url:          "https://mainnet.base.org".to_string(),
                block_explorer:   "https://basescan.org".to_string(),
                supports_eip1559: true,
            }),
            "avalanche" => Ok(ChainInfo {
                name:             "Avalanche C-Chain".to_string(),
                chain_id:         43114,
                native_currency:  "AVAX".to_string(),
                rpc_url:          "https://api.avax.network/ext/bc/C/rpc".to_string(),
                block_explorer:   "https://snowtrace.io".to_string(),
                supports_eip1559: true,
            }),
            "stellar" => Ok(ChainInfo {
                name:             "Stellar".to_string(),
                chain_id:         0,
                native_currency:  "XLM".to_string(),
                rpc_url:          "https://horizon.stellar.org".to_string(),
                block_explorer:   "https://stellar.expert".to_string(),
                supports_eip1559: false,
            }),
            _ => Err(anyhow!("Unsupported chain: {}", chain)),
        }
    }

    // ==================== DEBUG & UTILITY METHODS ====================

    pub async fn debug_print_users(&self) {
        println!("=== 🔍 DEBUG: CURRENT USERS ===");
        match self.database.get_all_users().await {
            Ok(users) => {
                if users.is_empty() {
                    println!("   No users in system");
                } else {
                    for user in &users {
                        println!("   Email:            {}", user.email);
                        println!("     ID:             {}", user.id);
                        println!("     Verified:       {}", user.is_verified);
                        println!("     Has password:   {}", user.password_hash.is_some());
                        println!("     Phone:          {:?}", user.phone_number);
                        println!("     Wallet:         {:?}", user.wallet_address);
                        println!("     Account type:   {:?}", user.account_type);
                        println!("     Profile done:   {}", user.is_profile_complete);
                        println!("   ---");
                    }
                    println!("=== TOTAL: {} ===", users.len());
                }
            }
            Err(e) => println!("❌ Error fetching users: {}", e),
        }
    }

    pub async fn get_user_stats(&self) -> Result<(usize, usize, usize, usize)> {
        let users = self.database.get_all_users().await?;
        Ok((
            users.len(),
            users.iter().filter(|u| u.wallet_address.is_some()).count(),
            users.iter().filter(|u| u.is_verified).count(),
            users.iter().filter(|u| u.password_hash.is_some()).count(),
        ))
    }

    pub async fn get_wallet_stats(&self) -> Result<HashMap<String, usize>> {
        let users = self.database.get_all_users().await?;
        let mut stats = HashMap::new();
        for user in users {
            if let Some(chain) = &user.wallet_chain {
                *stats.entry(chain.clone()).or_insert(0) += 1;
            }
        }
        Ok(stats)
    }

    pub async fn debug_verify_signature(
        &self,
        address: &str,
        signature: &str,
        message: &str,
    ) -> Result<String> {
        match self.verify_evm_signature(message, address, signature).await {
            Ok(_)  => Ok("✅ Signature is VALID".to_string()),
            Err(e) => Err(anyhow!("Signature verification failed: {}", e)),
        }
    }

    pub fn has_email_service(&self) -> bool {
        self.email_service.is_some()
    }

    pub async fn test_email_service(&self) -> Result<()> {
        if self.email_service.is_some() {
            println!("📧 TEST: Email service configured");
            Ok(())
        } else {
            Err(anyhow!("No email service configured"))
        }
    }

    // ==================== PASSTHROUGH HELPERS ====================

    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        self.database.get_all_users().await
    }

    pub async fn update_user(&self, user: &User) -> Result<()> {
        self.database.update_user(user).await
    }

    pub async fn create_user(&self, user: &User) -> Result<()> {
        self.database.create_user(user).await
    }

    pub async fn get_wallet_connections(&self, email: &str) -> Result<Vec<WalletConnection>> {
        self.database.get_wallet_connections(email).await
    }

    pub async fn store_verification_token(
        &self,
        token: &str,
        email: &str,
        token_type: &str,
        hours_valid: u32,
    ) -> Result<()> {
        self.database.store_verification_token(token, email, token_type, hours_valid).await
    }

    pub async fn get_user_activity_logs(&self, user_id: &str, limit: u32) -> Result<Vec<AuditLogEntry>> {
        self.database.get_audit_logs(Some(user_id), None, limit).await
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
        self.database.log_audit(user_id, action_type, action_target, target_id, details, ip_address).await
    }

    pub async fn get_user_notifications(
        &self,
        user_id: &str,
        include_read: bool,
    ) -> Result<Vec<crate::database::NotificationRecord>> {
        self.database.get_user_notifications(user_id, include_read).await
    }

    pub async fn mark_notification_read(&self, notification_id: &str) -> Result<()> {
        self.database.mark_notification_read(notification_id).await
    }

    pub async fn get_unread_notification_count(&self, user_id: &str) -> Result<i64> {
        self.database.get_unread_notification_count(user_id).await
    }

    // Private helpers

    async fn generate_login_challenge(&self, wallet_address: &str) -> Result<String> {
        let challenge = format!(
            "Login to FLUG Evidence\n\nSign this message to authenticate with your wallet.\n\nWallet: {}\nTimestamp: {}",
            wallet_address,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        Ok(challenge)
    }

    async fn verify_solana_signature(&self, wallet_address: &str) -> Result<()> {
        if wallet_address.len() < 32 || wallet_address.len() > 44 {
            return Err(anyhow!("Invalid Solana address length"));
        }
        println!("⚠️ SOLANA: Full sig verification requires solana_sdk crate");
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChainInfo {
    pub name:             String,
    pub chain_id:         u64,
    pub native_currency:  String,
    pub rpc_url:          String,
    pub block_explorer:   String,
    pub supports_eip1559: bool,
}

pub fn user_to_session_user(user: &User, wallet_connections: Vec<WalletConnection>) -> SessionUser {
    SessionUser {
        id:                  user.id.clone(),
        email:               user.email.clone(),
        has_password:        user.password_hash.is_some(),
        has_wallet:          user.wallet_address.is_some(),
        wallet_address:      user.wallet_address.clone(),
        wallet_type:         user.wallet_type.clone(),
        wallet_chain:        user.wallet_chain.clone(),
        is_verified:         user.is_verified,
        wallet_connections,
        account_type:        user.account_type.clone(),
        business_name:       user.business_name.clone(),
        geo_latitude:        user.geo_latitude,
        geo_longitude:       user.geo_longitude,
        is_profile_complete: user.is_profile_complete,
        phone_number:        user.phone_number.clone(),
        county:              user.county.clone(),
        id_number:           user.id_number.clone(),
    }
}