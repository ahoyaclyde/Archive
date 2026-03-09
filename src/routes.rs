// src/routes.rs
use actix_web::{web, HttpResponse, HttpRequest};
use actix_session::Session;
use serde::{Deserialize, Serialize};


use crate::auth::AuthService;
use crate::email_service::EmailService;

use crate::models::{
    ProfileCompletionForm,
    ProfileCompletionResponse,
    ProfileStatusResponse,

};

use std::collections::HashMap;

// ══════════════════════════════════════════════════════════════════════════════
// RESPONSE TYPES
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Serialize)]
pub struct ConnectWalletResponse {
    pub success:        bool,
    pub message:        String,
    pub wallet_address: Option<String>,
    pub redirect_to:    Option<String>,
}

#[derive(Serialize)]
pub struct WalletChallengeResponse {
    pub challenge: String,
    pub success:   bool,
}

#[derive(Serialize)]
pub struct WalletLoginResponse {
    pub success:              bool,
    pub message:              String,
    pub requires_registration: bool,
}

/// Generic JSON response used by the multi-step registration API endpoints.
#[derive(Serialize)]
pub struct ApiAuthResponse {
    pub success: bool,
    pub message: String,
}

// ══════════════════════════════════════════════════════════════════════════════
// REQUEST TYPES
// ══════════════════════════════════════════════════════════════════════════════

#[derive(Deserialize)]
pub struct LoginForm {
    pub email:    String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct ForgotPasswordForm {
    pub email: String,
}

#[derive(Deserialize)]
pub struct ResetPasswordForm {
    pub token:            String,
    pub password:         String,
    pub confirm_password: String,
}

#[derive(Deserialize)]
pub struct WalletChallengeRequest {
    pub email:          String,
    pub wallet_address: String,
    pub chain:          String,
}

#[derive(Deserialize)]
pub struct ConnectWalletForm {
    pub wallet_address: String,
    pub chain:          String,
    pub wallet_type:    String,
    pub signature:      String,
    pub public_key:     Option<String>,
}

#[derive(Serialize)]
pub struct WalletLoginFormData {
    pub wallet_address: String,
    pub chain:          String,
    pub message:        String,
}

#[derive(Deserialize)]
pub struct WalletLoginChallengeRequest {
    pub wallet_address: String,
}

#[derive(Deserialize)]
pub struct WalletLoginRequest {
    pub wallet_address: String,
    pub signature:      String,
    pub wallet_type:    String,
}

#[derive(Deserialize)]
pub struct WalletConnectRequest {
    pub wallet_type: String,
}

#[derive(Deserialize)]
pub struct WalletVerifyRequest {
    pub wallet_address: String,
    pub signature:      String,
    pub wallet_type:    String,
}

/// Step 1 of the multi-step registration form — email + phone.
#[derive(Deserialize)]
pub struct ApiRegisterRequest {
    pub email:        String,
    pub phone_number: String,
}

/// Step 2 of the multi-step registration form — password setup.
#[derive(Deserialize)]
pub struct ApiSetupPasswordRequest {
    pub email:    String,
    pub password: String,
}

// Query-string helpers
#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: Option<String>,
}

#[derive(Deserialize)]
pub struct EmailQuery {
    pub email: Option<String>,
}

// ══════════════════════════════════════════════════════════════════════════════
// TEMPLATE RENDERER
// ══════════════════════════════════════════════════════════════════════════════

fn render_auth_template(template_name: &str, context: HashMap<&str, String>) -> HttpResponse {
    let template_path = format!("static/templates/auth/{}.html", template_name);
    match std::fs::read_to_string(&template_path) {
        Ok(mut template) => {
            // ── Step 1: Process keys that ARE in the context ──────────────────
            for (key, value) in &context {
                let placeholder = format!("{{{{{}}}}}", key);
                template = template.replace(&placeholder, value);

                let if_start = format!("{{{{#if {}}}}}", key);
                let if_end   = "{{/if}}";

                if template.contains(&if_start) {
                    if !value.is_empty() && value != "false" && value != "0" {
                        // Truthy — strip only the tags, keep the content
                        template = template.replace(&if_start, "").replace(if_end, "");
                    } else {
                        // Falsy — remove the entire block including its content
                        while let Some(start) = template.find(&if_start) {
                            if let Some(rel_end) = template[start..].find(if_end) {
                                let block_end = start + rel_end + if_end.len();
                                template = format!("{}{}", &template[..start], &template[block_end..]);
                            } else {
                                break;
                            }
                        }
                    }
                }
            }

            // ── Step 2: Strip ALL remaining {{#if ...}}...{{/if}} blocks ─────
            // Any block whose key was not in the context is treated as falsy —
            // the entire block (tags + inner content) is removed so nothing
            // leaks onto the page when there is no error / value to show.
            let if_end = "{{/if}}";
            while let Some(start) = template.find("{{#if ") {
                if let Some(rel_end) = template[start..].find(if_end) {
                    let block_end = start + rel_end + if_end.len();
                    template = format!("{}{}", &template[..start], &template[block_end..]);
                } else {
                    // Malformed opening tag — remove just the tag to avoid an
                    // infinite loop, then let the next iteration continue.
                    if let Some(tag_end) = template[start..].find("}}") {
                        let end = start + tag_end + 2;
                        template = format!("{}{}", &template[..start], &template[end..]);
                    } else {
                        break;
                    }
                }
            }

            // ── Step 3: Clean up any remaining {{placeholder}} tags ───────────
            // Removes leftover {{key}} tokens whose keys were not supplied,
            // preventing raw template syntax from being sent to the browser.
            while let Some(start) = template.find("{{") {
                if let Some(rel_end) = template[start..].find("}}") {
                    let end = start + rel_end + 2;
                    template = format!("{}{}", &template[..start], &template[end..]);
                } else {
                    break;
                }
            }

            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(template)
        }
        Err(e) => {
            println!("❌ Error reading template {}: {}", template_path, e);
            HttpResponse::InternalServerError()
                .content_type("text/html; charset=utf-8")
                .body(format!(
                    "<h1>Error Loading Page</h1><p>Template '{}' not found: {}</p>",
                    template_name, e
                ))
        }
    }
}

/// Renders the check_email template — used by forgot-password flow only.
fn render_email_page(
    title:       &str,
    message:     &str,
    email:       &str,
    warning:     Option<&str>,
    action_url:  Option<&str>,
    manual_url:  Option<&str>,
    return_url:  &str,
    return_text: &str,
) -> HttpResponse {
    let mut context = HashMap::new();
    context.insert("title",   title.to_string());
    context.insert("message", message.to_string());
    context.insert("email",   email.to_string());

    if title.contains("Successful") || title.contains("Verified") || title.contains("Password Set") {
        context.insert("text_color",    "text-green-500".to_string());
        context.insert("icon_class",    "fas fa-check-circle text-2xl text-green-400".to_string());
        context.insert("icon_bg_color", "bg-green-900/30".to_string());
    } else if warning.is_some() {
        context.insert("text_color",    "text-yellow-500".to_string());
        context.insert("icon_class",    "fas fa-exclamation-triangle text-2xl text-yellow-400".to_string());
        context.insert("icon_bg_color", "bg-yellow-900/30".to_string());
    } else {
        context.insert("text_color",    "text-blue-500".to_string());
        context.insert("icon_class",    "fas fa-envelope text-2xl text-blue-400".to_string());
        context.insert("icon_bg_color", "bg-blue-900/30".to_string());
    }

    if let Some(warn) = warning {
        context.insert("warning",       warn.to_string());
        context.insert("warning_color", "text-yellow-400".to_string());
    }
    if let Some(url) = action_url {
        context.insert("action_url",  url.to_string());
        context.insert("action_text", "Continue".to_string());
    }
    if let Some(url) = manual_url {
        context.insert("manual_url",        url.to_string());
        context.insert("manual_text",       "Or copy this link:".to_string());
        context.insert("show_manual_link",  "true".to_string());
    }
    context.insert("return_url",  return_url.to_string());
    context.insert("return_text", return_text.to_string());

    render_auth_template("check_email", context)
}

// ══════════════════════════════════════════════════════════════════════════════
// PAGE ROUTES
// ══════════════════════════════════════════════════════════════════════════════

pub async fn index() -> HttpResponse {
    render_auth_template("index", HashMap::new())
}

/// Registration entry point — serves the multi-step register.html form.
/// All registration logic (email, phone, password) happens inside that form
/// via the /api/auth/* JSON endpoints below.
pub async fn register_form() -> HttpResponse {
    render_auth_template("register", HashMap::new())
}

/// Dashboard gate — checks session, profile completion, then routes accordingly.
pub async fn dashboard(session: Session, auth_service: web::Data<AuthService>) -> HttpResponse {
    println!("📊 DASHBOARD: Checking session...");

    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("📊 DASHBOARD: Session email: {}", email);

        match auth_service.get_user_by_email(&email).await {
            Ok(Some(user)) => {
                if !user.is_profile_complete {
                    println!("📊 DASHBOARD: Profile incomplete → /profile/complete");
                    return HttpResponse::SeeOther()
                        .append_header(("Location", "/profile/complete"))
                        .finish();
                }
                println!("📊 DASHBOARD: Profile complete → /evidence/dashboard");
            }
            Ok(None) => {
                println!("📊 DASHBOARD: User not in DB, clearing session");
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
            Err(e) => {
                println!("📊 DASHBOARD: DB error: {}", e);
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
        }

        HttpResponse::SeeOther()
            .append_header(("Location", "/evidence/dashboard"))
            .finish()
    } else {
        println!("📊 DASHBOARD: No session → /login");
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}

pub async fn login_form() -> HttpResponse {
    render_auth_template("login", HashMap::new())
}

/// Password login handler.
/// On success redirects to /dashboard (not /evidence/dashboard directly) so
/// the profile-completion gate always fires correctly.
pub async fn login(
    form:         web::Form<LoginForm>,
    auth_service: web::Data<AuthService>,
    session:      Session,
) -> HttpResponse {
    match auth_service.verify_password(&form.email, &form.password).await {
        Ok(Some(user)) => {
            let _ = session.insert("user_id",    &user.id);
            let _ = session.insert("user_email", &user.email);
            println!("🎉 LOGIN: Successful for {} → /dashboard", user.email);
            HttpResponse::SeeOther()
                .append_header(("Location", "/dashboard"))
                .finish()
        }
        _ => {
            let mut context = HashMap::new();
            context.insert("error_message", "Invalid email or password. Please try again.".to_string());
            render_auth_template("login", context)
        }
    }
}

pub async fn logout(session: Session) -> HttpResponse {
    let _ = session.clear();
    HttpResponse::SeeOther()
        .append_header(("Location", "/login"))
        .finish()
}

// ── Email verification (deferred — user triggers this from their profile) ─────
// The route still works so links in verification emails that were already sent
// continue to function.

pub async fn verify_email_form(query: web::Query<TokenQuery>) -> HttpResponse {
    let token = query.token.as_deref().unwrap_or("");
    println!("🔍 VERIFY_EMAIL_FORM: token={}", token);
    let mut context = HashMap::new();
    context.insert("token", token.to_string());
    render_auth_template("verify_email", context)
}

pub async fn verify_email_handler(
    form:          web::Form<crate::routes::VerifyEmailFormBody>,
    auth_service:  web::Data<AuthService>,
) -> HttpResponse {
    match auth_service.verify_email(&form.token).await {
        Ok(user) => {
            let mut context = HashMap::new();
            context.insert("email",      user.email.clone());
            context.insert("email_sent", "false".to_string());
            render_auth_template("verify_email_success", context)
        }
        Err(e) => {
            let mut context = HashMap::new();
            context.insert("error_message", format!("Verification failed: {}", e));
            render_auth_template("verify_email", context)
        }
    }
}

#[derive(Deserialize)]
pub struct VerifyEmailFormBody {
    pub token: String,
}

// ── Profile completion page (standalone — not part of multi-step form) ─────────

pub async fn profile_completion_page(session: Session) -> HttpResponse {
    if session.get::<String>("user_email").unwrap_or(None).is_some() {
        render_auth_template("profile_complete", HashMap::new())
    } else {
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}

// ── Wallet pages ───────────────────────────────────────────────────────────────

pub async fn connect_wallet_page(session: Session) -> HttpResponse {
    let user_email = session.get::<String>("user_email").unwrap_or(None);
    let mut context = HashMap::new();

    if let Some(email) = &user_email {
        context.insert("page_title", "Connect Wallet to Your Account".to_string());
        context.insert("message",    format!("Connect wallet to account: {}", email));
        context.insert("is_logged_in", "true".to_string());
    } else {
        context.insert("page_title",   "Connect Wallet".to_string());
        context.insert("message",      "Login or create account with your wallet".to_string());
        context.insert("is_logged_in", "false".to_string());
    }

    render_auth_template("connect_wallet", context)
}

pub async fn wallet_login_page() -> HttpResponse {
    render_auth_template("wallet_login", HashMap::new())
}

// ── Password reset (kept — separate from registration flow) ────────────────────

pub async fn forgot_password_page() -> HttpResponse {
    render_auth_template("forgot_password", HashMap::new())
}

pub async fn forgot_password_handler(
    form:          web::Form<ForgotPasswordForm>,
    auth_service:  web::Data<AuthService>,
    email_service: Option<web::Data<EmailService>>,
) -> HttpResponse {
    match auth_service.request_password_reset(&form.email).await {
        Ok(msg) => {
            let has_email_service = email_service.is_some();
            if has_email_service && !msg.contains("Manual") {
                render_email_page(
                    "Check Your Email",
                    "We've sent a password reset link to your email address.",
                    &form.email,
                    None, None, None,
                    "/login",
                    "Return to Sign In",
                )
            } else {
                render_email_page(
                    "Reset Link Generated",
                    "Use the link below to reset your password.",
                    &form.email,
                    Some("Email service unavailable — use the manual link below."),
                    None,
                    Some(&msg),
                    "/login",
                    "Return to Sign In",
                )
            }
        }
        Err(e) => {
            let mut context = HashMap::new();
            context.insert("error_message", format!("Error: {}", e));
            render_auth_template("forgot_password", context)
        }
    }
}

pub async fn reset_password_page(request: HttpRequest) -> HttpResponse {
    let token = request.query_string().strip_prefix("token=").unwrap_or("");
    let mut context = HashMap::new();
    context.insert("token", token.to_string());
    render_auth_template("reset_password", context)
}

pub async fn reset_password_handler(
    form:         web::Form<ResetPasswordForm>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    if form.password != form.confirm_password {
        let mut context = HashMap::new();
        context.insert("token",         form.token.clone());
        context.insert("error_message", "Passwords do not match".to_string());
        return render_auth_template("reset_password", context);
    }
    if form.password.len() < 6 {
        let mut context = HashMap::new();
        context.insert("token",         form.token.clone());
        context.insert("error_message", "Password must be at least 6 characters".to_string());
        return render_auth_template("reset_password", context);
    }

    match auth_service.reset_password(&form.token, &form.password).await {
        Ok(_) => render_auth_template("reset_password_success", HashMap::new()),
        Err(e) => {
            let mut context = HashMap::new();
            context.insert("token",         form.token.clone());
            context.insert("error_message", format!("Failed to reset password: {}", e));
            render_auth_template("reset_password", context)
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// JSON API — MULTI-STEP REGISTRATION FORM
// ══════════════════════════════════════════════════════════════════════════════

/// POST /api/auth/register
/// Multi-step form Step 1: accepts { email, phone_number }, creates the account.
/// Does NOT require a session — this is the very first call a new user makes.
pub async fn api_auth_register(
    request:      web::Json<ApiRegisterRequest>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let email = request.email.trim().to_lowercase();
    let phone = request.phone_number.trim().replace(' ', "");

    println!("📝 API_AUTH_REGISTER: email={} phone={}", email, phone);

    // Server-side validation (client already validates, but never trust the client)
    let email_rx = match regex::Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$") {
        Ok(r) => r,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiAuthResponse {
                success: false,
                message: "Server configuration error".to_string(),
            });
        }
    };
    if !email_rx.is_match(&email) {
        return HttpResponse::BadRequest().json(ApiAuthResponse {
            success: false,
            message: "Invalid email address.".to_string(),
        });
    }

    let phone_rx = match regex::Regex::new(r"^(07\d{8}|011\d{7}|\+2547\d{8}|\+25411\d{7})$") {
        Ok(r) => r,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ApiAuthResponse {
                success: false,
                message: "Server configuration error".to_string(),
            });
        }
    };
    if !phone_rx.is_match(&phone) {
        return HttpResponse::BadRequest().json(ApiAuthResponse {
            success: false,
            message: "Invalid Kenyan phone number. Use format: 0712345678 or +254712345678".to_string(),
        });
    }

    match auth_service.register_user(&email, Some(&phone)).await {
        Ok(_) => {
            println!("✅ API_AUTH_REGISTER: Success for {}", email);
            HttpResponse::Ok().json(ApiAuthResponse {
                success: true,
                message: "Account created. Set your password to continue.".to_string(),
            })
        }
        Err(e) => {
            let msg = e.to_string();
            println!("❌ API_AUTH_REGISTER: Failed for {} — {}", email, msg);
            let user_msg = if msg.contains("already exists") {
                "An account with this email already exists. Please sign in instead.".to_string()
            } else {
                format!("Registration failed: {}", msg)
            };
            HttpResponse::BadRequest().json(ApiAuthResponse {
                success: false,
                message: user_msg,
            })
        }
    }
}

/// POST /api/auth/setup-password
/// Multi-step form Step 2: accepts { email, password }, hashes and stores it,
/// then creates a session so the user is immediately authenticated.
pub async fn api_auth_setup_password(
    session:      Session,
    request:      web::Json<ApiSetupPasswordRequest>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let email    = request.email.trim().to_lowercase();
    let password = request.password.clone();

    println!("🔐 API_AUTH_SETUP_PASSWORD: email={}", email);

    if password.len() < 6 {
        return HttpResponse::BadRequest().json(ApiAuthResponse {
            success: false,
            message: "Password must be at least 6 characters.".to_string(),
        });
    }

    match auth_service.set_password(&email, &password).await {
        Ok(()) => {
            // Create session immediately — user is logged in after setting their password
            match auth_service.get_user_by_email(&email).await {
                Ok(Some(user)) => {
                    let _ = session.insert("user_id",    &user.id);
                    let _ = session.insert("user_email", &user.email);
                    println!("✅ API_AUTH_SETUP_PASSWORD: Session created for {}", email);
                    HttpResponse::Ok().json(ApiAuthResponse {
                        success: true,
                        message: "Password set successfully. Your account is ready.".to_string(),
                    })
                }
                _ => {
                    // Password was saved but session creation failed — still a success;
                    // user will need to sign in manually.
                    println!("⚠️ API_AUTH_SETUP_PASSWORD: Password set but session creation failed for {}", email);
                    HttpResponse::Ok().json(ApiAuthResponse {
                        success: true,
                        message: "Password set. Please sign in to continue.".to_string(),
                    })
                }
            }
        }
        Err(e) => {
            println!("❌ API_AUTH_SETUP_PASSWORD: Failed for {} — {}", email, e);
            HttpResponse::BadRequest().json(ApiAuthResponse {
                success: false,
                message: format!("Failed to set password: {}", e),
            })
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// JSON API — PROFILE
// ══════════════════════════════════════════════════════════════════════════════

pub async fn api_profile_complete_kenya(
    session:      Session,
    form:         web::Form<ProfileCompletionForm>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    if let Some(current_email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("📝 API_PROFILE_COMPLETE: email={}", current_email);

        let profile_data = form.into_inner();
        let is_wallet_user = current_email.starts_with("wallet_")
                          || current_email.contains("@flug.evidence");

        let email_clone = profile_data.email.clone();
        let new_email   = if is_wallet_user { email_clone.as_deref() } else { None };

        if let Some(email) = new_email {
            if email.trim().is_empty() {
                return HttpResponse::BadRequest().json(ProfileCompletionResponse {
                    success: false,
                    message: "Email is required for wallet users".to_string(),
                    redirect_to: None,
                });
            }
            if !email.contains('@') || !email.contains('.') {
                return HttpResponse::BadRequest().json(ProfileCompletionResponse {
                    success: false,
                    message: "Invalid email format".to_string(),
                    redirect_to: None,
                });
            }
        }

        match auth_service.complete_profile(&current_email, profile_data, new_email).await {
            Ok(user) => {
                if let Some(new_email_val) = new_email {
                    if new_email_val != current_email {
                        let _ = session.insert("user_email", user.email.clone());
                        println!("✅ Session email updated to '{}'", new_email_val);
                    }
                }
                HttpResponse::Ok().json(ProfileCompletionResponse {
                    success:     true,
                    message:     "Profile completed successfully.".to_string(),
                    redirect_to: Some("/evidence/dashboard".to_string()),
                })
            }
            Err(e) => HttpResponse::BadRequest().json(ProfileCompletionResponse {
                success:     false,
                message:     format!("Failed to complete profile: {}", e),
                redirect_to: None,
            }),
        }
    } else {
        HttpResponse::Unauthorized().json(ProfileCompletionResponse {
            success:     false,
            message:     "Not authenticated".to_string(),
            redirect_to: Some("/login".to_string()),
        })
    }
}

pub async fn api_profile_status(
    session:      Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        match auth_service.check_profile_status(&email).await {
            Ok(status) => HttpResponse::Ok().json(status),
            Err(_)     => HttpResponse::InternalServerError().json(ProfileStatusResponse {
                is_profile_complete:  false,
                missing_fields:       vec!["account_type".to_string()],
                current_account_type: None,
            }),
        }
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// JSON API — WALLET
// ══════════════════════════════════════════════════════════════════════════════

pub async fn connect_wallet_handler(
    session:       Session,
    form:          web::Form<ConnectWalletForm>,
    auth_service:  web::Data<AuthService>,
    email_service: Option<web::Data<EmailService>>,
) -> HttpResponse {
    println!("🔗 CONNECT_WALLET_HANDLER: wallet={} chain={}", form.wallet_address, form.chain);

    // CASE 1: User is already logged in — attach wallet to existing account
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("   Attaching wallet to existing account: {}", email);

        match auth_service.connect_wallet(
            &email,
            &form.wallet_address,
            &form.chain,
            &form.wallet_type,
            &form.signature,
            form.public_key.as_deref(),
        ).await {
            Ok(_) => {
                let _ = session.insert("wallet_address", &form.wallet_address);
                let _ = session.insert("wallet_chain",   &form.chain);
                let _ = session.insert("wallet_type",    &form.wallet_type);

                if let Some(svc) = email_service {
                    let _ = svc.send_wallet_connected_email(&email, &form.wallet_address, &form.chain).await;
                }

                HttpResponse::Ok().json(ConnectWalletResponse {
                    success:        true,
                    message:        "Wallet connected to your account.".to_string(),
                    wallet_address: Some(form.wallet_address.clone()),
                    redirect_to:    Some("/evidence/dashboard".to_string()),
                })
            }
            Err(e) => HttpResponse::BadRequest().json(ConnectWalletResponse {
                success:        false,
                message:        format!("Failed: {}", e),
                wallet_address: None,
                redirect_to:    None,
            }),
        }
    }
    // CASE 2: Not logged in — check if wallet has an account, or register a new one
    else {
        println!("   No session — checking wallet registry");

        match auth_service.find_user_by_wallet(&form.wallet_address).await {
            // Wallet already has an account — log them in
            Ok(Some(user)) => {
                println!("   Wallet found — logging in: {}", user.email);
                let _ = session.insert("user_id",        &user.id);
                let _ = session.insert("user_email",     &user.email);
                let _ = session.insert("wallet_address", &form.wallet_address);
                let _ = session.insert("wallet_chain",   &form.chain);
                let _ = session.insert("wallet_type",    &form.wallet_type);

                let redirect = if !user.is_profile_complete {
                    "/profile/complete".to_string()
                } else {
                    "/evidence/dashboard".to_string()
                };

                HttpResponse::Ok().json(ConnectWalletResponse {
                    success:        true,
                    message:        "Login successful.".to_string(),
                    wallet_address: Some(form.wallet_address.clone()),
                    redirect_to:    Some(redirect),
                })
            }
            // No account — register a new wallet-only account
            Ok(None) => {
                println!("   Wallet not found — registering new account");

                match auth_service.register_with_wallet(
                    &form.wallet_address,
                    &form.chain,
                    &form.wallet_type,
                    &form.signature,
                    form.public_key.as_deref(),
                ).await {
                    Ok(user) => {
                        let _ = session.insert("user_id",        &user.id);
                        let _ = session.insert("user_email",     &user.email);
                        let _ = session.insert("wallet_address", &form.wallet_address);
                        let _ = session.insert("wallet_chain",   &form.chain);
                        let _ = session.insert("wallet_type",    &form.wallet_type);

                        HttpResponse::Ok().json(ConnectWalletResponse {
                            success:        true,
                            message:        "Account created! Complete your profile to continue.".to_string(),
                            wallet_address: Some(form.wallet_address.clone()),
                            redirect_to:    Some("/profile/complete".to_string()),
                        })
                    }
                    Err(e) => HttpResponse::BadRequest().json(ConnectWalletResponse {
                        success:        false,
                        message:        format!("Registration failed: {}", e),
                        wallet_address: None,
                        redirect_to:    None,
                    }),
                }
            }
            Err(e) => HttpResponse::BadRequest().json(ConnectWalletResponse {
                success:        false,
                message:        format!("Error: {}", e),
                wallet_address: None,
                redirect_to:    None,
            }),
        }
    }
}

pub async fn disconnect_wallet(
    session:      Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        match auth_service.disconnect_wallet(&email).await {
            Ok(_) => {
                let _ = session.remove("wallet_address");
                let _ = session.remove("wallet_chain");
                let _ = session.remove("wallet_type");
                HttpResponse::Ok().json(serde_json::json!({ "success": true }))
            }
            Err(e) => HttpResponse::BadRequest().json(serde_json::json!({
                "success": false, "message": format!("{}", e)
            })),
        }
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

pub async fn generate_wallet_challenge(
    session:      Session,
    auth_service: web::Data<AuthService>,
    request:      web::Json<WalletChallengeRequest>,
) -> HttpResponse {
    if let Some(user_email) = session.get::<String>("user_email").unwrap_or(None) {
        if user_email != request.email {
            return HttpResponse::Unauthorized().json(WalletChallengeResponse {
                challenge: String::new(),
                success:   false,
            });
        }
        match auth_service.generate_wallet_connection_challenge(
            &request.email,
            &request.wallet_address,
            &request.chain,
        ).await {
            Ok(challenge) => HttpResponse::Ok().json(WalletChallengeResponse { challenge, success: true }),
            Err(_)        => HttpResponse::BadRequest().json(WalletChallengeResponse {
                challenge: String::new(), success: false,
            }),
        }
    } else {
        HttpResponse::Unauthorized().json(WalletChallengeResponse {
            challenge: String::new(), success: false,
        })
    }
}

pub async fn api_wallet_connection_challenge(
    session:      Session,
    auth_service: web::Data<AuthService>,
    request:      web::Json<WalletChallengeRequest>,
) -> HttpResponse {
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        match auth_service.generate_wallet_connection_challenge(
            &email,
            &request.wallet_address,
            &request.chain,
        ).await {
            Ok(challenge) => HttpResponse::Ok().json(WalletChallengeResponse { challenge, success: true }),
            Err(_)        => HttpResponse::BadRequest().json(WalletChallengeResponse {
                challenge: String::new(), success: false,
            }),
        }
    } else {
        HttpResponse::Unauthorized().json(WalletChallengeResponse {
            challenge: String::new(), success: false,
        })
    }
}

pub async fn generate_wallet_login_challenge(
    request: web::Json<WalletLoginChallengeRequest>,
) -> HttpResponse {
    let challenge = format!(
        "Sign in to FLUG Evidence\n\nWallet: {}\nTimestamp: {}",
        request.wallet_address,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    HttpResponse::Ok().json(WalletChallengeResponse { challenge, success: true })
}

pub async fn wallet_login(
    auth_service: web::Data<AuthService>,
    session:      Session,
    request:      web::Json<WalletLoginRequest>,
) -> HttpResponse {
    match auth_service.login_with_wallet(
        &request.wallet_address,
        &request.signature,
        &request.wallet_type,
    ).await {
        Ok(Some(user)) => {
            let _ = session.insert("user_id",    &user.id);
            let _ = session.insert("user_email", &user.email);
            if let Some(ref addr) = user.wallet_address {
                let _ = session.insert("wallet_address", addr);
            }
            if let Some(ref chain) = user.wallet_chain {
                let _ = session.insert("wallet_chain", chain);
            }
            if let Some(ref wtype) = user.wallet_type {
                let _ = session.insert("wallet_type", wtype);
            }
            HttpResponse::Ok().json(WalletLoginResponse {
                success:               true,
                message:               "Login successful".to_string(),
                requires_registration: false,
            })
        }
        Ok(None) => HttpResponse::Ok().json(WalletLoginResponse {
            success:               false,
            message:               "No account found for this wallet".to_string(),
            requires_registration: true,
        }),
        Err(e) => HttpResponse::BadRequest().json(WalletLoginResponse {
            success:               false,
            message:               format!("{}", e),
            requires_registration: false,
        }),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// MISC
// ══════════════════════════════════════════════════════════════════════════════

pub async fn test_email_handler(email_service: web::Data<EmailService>) -> HttpResponse {
    match email_service.send_verification_email("test@example.com", "http://localhost:8080/verify-email?token=test").await {
        Ok(_)  => HttpResponse::Ok().body("✅ Test email sent successfully!"),
        Err(e) => HttpResponse::InternalServerError().body(format!("❌ Test email failed: {}", e)),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// ROUTE CONFIGURATION
// ══════════════════════════════════════════════════════════════════════════════

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        // ── Pages ──────────────────────────────────────────────────────────
        .route("/",               web::get().to(index))
        .route("/register",       web::get().to(register_form))      // multi-step form
        .route("/login",          web::get().to(login_form))
        .route("/login",          web::post().to(login))
        .route("/logout",         web::post().to(logout))
        .route("/dashboard",      web::get().to(dashboard))          // profile-completion gate
        .route("/verify-email",   web::get().to(verify_email_form))  // deferred verification
        .route("/verify-email",   web::post().to(verify_email_handler))
        .route("/forgot-password", web::get().to(forgot_password_page))
        .route("/forgot-password", web::post().to(forgot_password_handler))
        .route("/reset-password",  web::get().to(reset_password_page))
        .route("/reset-password",  web::post().to(reset_password_handler))
        .route("/profile/complete", web::get().to(profile_completion_page))
        .route("/wallet-login",    web::get().to(wallet_login_page))
        .route("/connect-wallet",  web::get().to(connect_wallet_page))
        .route("/connect-wallet",  web::post().to(connect_wallet_handler))
        .route("/disconnect-wallet", web::post().to(disconnect_wallet))

        // ── Registration API (multi-step form) ─────────────────────────────
        .route("/api/auth/register",        web::post().to(api_auth_register))
        .route("/api/auth/setup-password",  web::post().to(api_auth_setup_password))

        // ── Profile API ────────────────────────────────────────────────────
        .route("/api/profile/complete",       web::post().to(api_profile_complete_kenya))
        .route("/api/profile/complete-kenya", web::post().to(api_profile_complete_kenya))
        .route("/api/profile/status",         web::get().to(api_profile_status))

        // ── Wallet API ─────────────────────────────────────────────────────
        .route("/api/wallet/connection-challenge", web::post().to(api_wallet_connection_challenge))
        .route("/api/wallet/login-challenge",      web::post().to(generate_wallet_login_challenge))
        .route("/api/wallet/login",                web::post().to(wallet_login))

        // ── Dev/test ───────────────────────────────────────────────────────
        .route("/test-email", web::get().to(test_email_handler));
}