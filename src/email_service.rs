// src/email_service.rs
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json;

#[derive(Debug, Clone)]
pub struct EmailService {
    api_key: String,
    from_email: String,
    from_name: String,
    client: Client,
}

impl EmailService {
    pub fn new() -> Result<Self> {
        dotenv::dotenv().ok();
        
        let api_key = std::env::var("RESEND_API_KEY")
            .map_err(|_| anyhow!("RESEND_API_KEY not set"))?;
        
        let from_email = std::env::var("RESEND_FROM_EMAIL")
            .unwrap_or_else(|_| "onboarding@resend.dev".to_string());
        
        let from_name = std::env::var("RESEND_FROM_NAME")
            .unwrap_or_else(|_| "FLUG Evidence".to_string());

        Ok(Self {
            api_key,
            from_email,
            from_name,
            client: Client::new(),
        })
    }

    

    pub async fn send_welcome_email(&self, email: &str) -> Result<()> {
    let subject = "Welcome to FLUG Evidence - Get Started";
    
    let html_content = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Welcome to FLUG Evidence</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .logo {{
            font-size: 32px;
            font-weight: bold;
            color: #E50914;
            margin-bottom: 10px;
        }}
        .card {{
            background: #f8f9fa;
            border-radius: 10px;
            padding: 30px;
            margin: 20px 0;
            border: 1px solid #e9ecef;
        }}
        .feature {{
            background: #e3f2fd;
            border-left: 4px solid #2196F3;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
        .cta-button {{
            display: inline-block;
            background: #E50914;
            color: white;
            padding: 12px 30px;
            text-decoration: none;
            border-radius: 5px;
            font-weight: bold;
            margin: 20px 0;
            text-align: center;
        }}
        .steps {{
            counter-reset: step-counter;
        }}
        .step {{
            margin: 20px 0;
            padding-left: 50px;
            position: relative;
        }}
        .step::before {{
            counter-increment: step-counter;
            content: counter(step-counter);
            position: absolute;
            left: 0;
            top: 0;
            background: #E50914;
            color: white;
            width: 30px;
            height: 30px;
            border-radius: 50%;
            display: flex;
            align-items: center;
            justify-content: center;
            font-weight: bold;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
            text-align: center;
        }}
        .highlight {{
            background: #fff3cd;
            border-left: 4px solid #ffc107;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">FLUG Evidence</div>
        <div style="color: #6c757d; font-size: 14px;">Decentralized Media Platform</div>
    </div>
    
    <div class="card">
        <h1 style="margin-top: 0;">Welcome to FLUG Evidence! 🎉</h1>
        
        <p>Hello, and welcome to FLUG Evidence! We're excited to have you join our community of creators who value authenticity, ownership, and decentralization.</p>
        
        <div class="feature">
            <p><strong>🚀 Your account has been successfully verified!</strong></p>
            <p>You now have full access to all FLUG Evidence features.</p>
        </div>
        
        <h2>What is FLUG Evidence?</h2>
        <p>FLUG Evidence is a next-generation platform that combines media storage with blockchain technology to provide:</p>
        <ul>
            <li>🔒 <strong>Secure, permanent storage</strong> on decentralized networks</li>
            <li>⚡ <strong>Blockchain verification</strong> of content authenticity</li>
            <li>🎯 <strong>Cryptographic signatures</strong> to prove ownership</li>
            <li>🌐 <strong>Public sharing</strong> with permanent, tamper-proof URLs</li>
        </ul>
        
        <h2>Get Started in 3 Easy Steps:</h2>
        <div class="steps">
            <div class="step">
                <h3>Upload Your First Media</h3>
                <p>Start by uploading videos, images, or documents. Your content is automatically backed up on decentralized storage.</p>
            </div>
            
            <div class="step">
                <h3>Connect Your Wallet</h3>
                <p>Link your blockchain wallet (Ethereum, Solana, Stellar, etc.) to cryptographically sign your content and prove ownership.</p>
            </div>
            
            <div class="step">
                <h3>Share with Confidence</h3>
                <p>Share your content with permanent, verifiable links. Anyone can verify the authenticity and ownership of your media.</p>
            </div>
        </div>
        
        <div style="text-align: center;">
            <a href="http://localhost:8080/dashboard" class="cta-button">Go to Your Dashboard</a>
        </div>
        
        <div class="highlight">
            <p><strong>🎁 Bonus: Blockchain Signing</strong></p>
            <p>Connect a wallet to unlock blockchain signing features. This allows you to create permanent, tamper-proof records of your content on the blockchain.</p>
        </div>
        
        <h2>Need Help?</h2>
        <ul>
            <li>📖 Check out our <a href="http://localhost:8080/docs">documentation</a></li>
            <li>🎥 Watch our <a href="http://localhost:8080/tutorials">video tutorials</a></li>
            <li>💬 Join our <a href="http://localhost:8080/community">community forum</a></li>
            <li>📧 Contact <a href="mailto:support@flugevidence.com">support@flugevidence.com</a></li>
        </ul>
        
        <p>We're committed to building a platform that puts creators first. We can't wait to see what you'll create!</p>
        
        <p>Best regards,<br>
        <strong>The FLUG Evidence Team</strong></p>
    </div>
    
    <div class="footer">
        <p>© 2024 FLUG Evidence. All rights reserved.</p>
        <p>This email was sent to {email} as part of your FLUG Evidence account registration.</p>
        <p>FLUG Evidence | Decentralized Media Platform</p>
        <p>If you believe this email was sent in error, please contact us immediately.</p>
    </div>
</body>
</html>
    "#);

    let text_content = format!(r#"
Welcome to FLUG Evidence! 🎉

Hello, and welcome to FLUG Evidence! We're excited to have you join our community of creators who value authenticity, ownership, and decentralization.

🚀 Your account has been successfully verified!
You now have full access to all FLUG Evidence features.

What is FLUG Evidence?
FLUG Evidence is a next-generation platform that combines media storage with blockchain technology to provide:
- 🔒 Secure, permanent storage on decentralized networks
- ⚡ Blockchain verification of content authenticity
- 🎯 Cryptographic signatures to prove ownership
- 🌐 Public sharing with permanent, tamper-proof URLs

Get Started in 3 Easy Steps:
1. Upload Your First Media
   Start by uploading videos, images, or documents. Your content is automatically backed up on decentralized storage.

2. Connect Your Wallet
   Link your blockchain wallet (Ethereum, Solana, Stellar, etc.) to cryptographically sign your content and prove ownership.

3. Share with Confidence
   Share your content with permanent, verifiable links. Anyone can verify the authenticity and ownership of your media.

🎁 Bonus: Blockchain Signing
Connect a wallet to unlock blockchain signing features. This allows you to create permanent, tamper-proof records of your content on the blockchain.

Need Help?
- 📖 Check out our documentation: http://localhost:8080/docs
- 🎥 Watch our video tutorials: http://localhost:8080/tutorials
- 💬 Join our community forum: http://localhost:8080/community
- 📧 Contact support: support@flugevidence.com

We're committed to building a platform that puts creators first. We can't wait to see what you'll create!

Best regards,
The FLUG Evidence Team

© 2024 FLUG Evidence. All rights reserved.
This email was sent to {email} as part of your FLUG Evidence account registration.
FLUG Evidence | Decentralized Media Platform
If you believe this email was sent in error, please contact us immediately.
    "#);

    println!("📧 Sending welcome email to: {}", email);
    self.send_email(email, subject, &html_content, &text_content).await
}
   


     pub fn new_test_mode() -> Self {
        println!("📧 Email service in TEST MODE - emails will be logged but not sent");
        Self {
            api_key: "test_mode".to_string(),
            from_email: "test@flug.evidence".to_string(),
            from_name: "FLUG Evidence Test".to_string(),
            client: Client::new(),
        }
    }


       // Update the send_email method to handle test mode:
    async fn send_email(&self, to_email: &str, subject: &str, html_content: &str, text_content: &str) -> Result<()> {
        // Check if we're in test mode
        if self.api_key == "test_mode" {
            println!("📧 [TEST MODE] Would send email to: {}", to_email);
            println!("   Subject: {}", subject);
            println!("   Preview: {}...", &html_content[..html_content.len().min(200)]);
            println!("   ---");
            return Ok(());
        }
        
        println!("📧 Attempting to send email to: {}", to_email);
        
        let request_body = serde_json::json!({
            "from": format!("{} <{}>", self.from_name, self.from_email),
            "to": to_email,
            "subject": subject,
            "html": html_content,
            "text": text_content,
        });

        let response = self.client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to send request to Resend: {}", e))?;

        let status = response.status();
        
        if status.is_success() {
            let response_body: serde_json::Value = response
                .json()
                .await
                .map_err(|e| anyhow!("Failed to parse Resend response: {}", e))?;
            
            println!("✅ Email sent successfully! Response: {:?}", response_body);
            Ok(())
        } else {
            // Get the error text first, then use the status
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            println!("❌ Failed to send email. Status: {}, Error: {}", status, error_text);
            Err(anyhow!("Resend API error: Status {}, Body: {}", status, error_text))
        }
    }


    pub async fn send_verification_email(&self, to_email: &str, verification_url: &str) -> Result<()> {
        let subject = "Verify Your FLUG Evidence Account";
        
        let html_content = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Verify Your Account</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .logo {{
            font-size: 28px;
            font-weight: bold;
            color: #E50914;
            margin-bottom: 10px;
        }}
        .card {{
            background: #f8f9fa;
            border-radius: 10px;
            padding: 30px;
            margin: 20px 0;
            border: 1px solid #e9ecef;
        }}
        .button {{
            display: inline-block;
            background: #E50914;
            color: white;
            text-decoration: none;
            padding: 12px 30px;
            border-radius: 5px;
            font-weight: bold;
            margin: 20px 0;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
            text-align: center;
        }}
        .verification-code {{
            font-family: monospace;
            background: #e9ecef;
            padding: 10px;
            border-radius: 5px;
            margin: 10px 0;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">FLUG Evidence</div>
        <div style="color: #6c757d; font-size: 14px;">Decentralized Media Platform</div>
    </div>
    
    <div class="card">
        <h1 style="margin-top: 0;">Welcome to FLUG Evidence!</h1>
        
        <p>Thank you for registering with FLUG Evidence. To complete your account setup, please verify your email address by clicking the button below:</p>
        
        <div style="text-align: center;">
            <a href="{verification_url}" class="button">Verify Email Address</a>
        </div>
        
        <p>Or copy and paste this link into your browser:</p>
        <div class="verification-code">{verification_url}</div>
        
        <p>This verification link will expire in 24 hours.</p>
        
        <hr style="margin: 25px 0; border: none; border-top: 1px solid #e9ecef;">
        
        <p><strong>What's next?</strong></p>
        <ul>
            <li>Upload and cryptographically sign your media content</li>
            <li>Connect your blockchain wallet for content verification</li>
            <li>Browse decentralized content from other creators</li>
            <li>Build your personalized media library</li>
        </ul>
        
        <p>If you have any questions, feel free to reply to this email.</p>
    </div>
    
    <div class="footer">
        <p>© 2024 FLUG Evidence. All rights reserved.</p>
        <p>This email was sent to {to_email}</p>
        <p>If you didn't create an account with FLUG Evidence, please ignore this email.</p>
    </div>
</body>
</html>
        "#);

        let text_content = format!(r#"
Welcome to FLUG Evidence!

Thank you for registering with FLUG Evidence. To complete your account setup, please verify your email address.

Verification URL: {verification_url}

Copy and paste this link into your browser to verify your email address. This link will expire in 24 hours.

What's next?
- Upload and cryptographically sign your media content
- Connect your blockchain wallet for content verification
- Browse decentralized content from other creators
- Build your personalized media library

If you have any questions, feel free to reply to this email.

© 2024 FLUG Evidence. All rights reserved.
This email was sent to {to_email}
If you didn't create an account with FLUG Evidence, please ignore this email.
        "#);

        self.send_email(to_email, subject, &html_content, &text_content).await
    }

    pub async fn send_password_setup_email(&self, to_email: &str, setup_url: &str) -> Result<()> {
        let subject = "Set Your FLUG Evidence Password";
        
        let html_content = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Set Your Password</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .logo {{
            font-size: 28px;
            font-weight: bold;
            color: #E50914;
            margin-bottom: 10px;
        }}
        .card {{
            background: #f8f9fa;
            border-radius: 10px;
            padding: 30px;
            margin: 20px 0;
            border: 1px solid #e9ecef;
        }}
        .button {{
            display: inline-block;
            background: #E50914;
            color: white;
            text-decoration: none;
            padding: 12px 30px;
            border-radius: 5px;
            font-weight: bold;
            margin: 20px 0;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
            text-align: center;
        }}
        .security-tips {{
            background: #e8f4fd;
            border-left: 4px solid #2196F3;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">FLUG Evidence</div>
        <div style="color: #6c757d; font-size: 14px;">Decentralized Media Platform</div>
    </div>
    
    <div class="card">
        <h1 style="margin-top: 0;">Set Your Password</h1>
        
        <p>Your email has been verified successfully! Now it's time to set up your account password.</p>
        
        <div style="text-align: center;">
            <a href="{setup_url}" class="button">Set Password</a>
        </div>
        
        <p>Or copy and paste this link into your browser:</p>
        <div style="font-family: monospace; background: #e9ecef; padding: 10px; border-radius: 5px; margin: 10px 0; word-break: break-all;">
            {setup_url}
        </div>
        
        <div class="security-tips">
            <p><strong>Security Tips:</strong></p>
            <ul style="margin: 10px 0; padding-left: 20px;">
                <li>Use a strong, unique password</li>
                <li>Include uppercase, lowercase, numbers, and symbols</li>
                <li>Avoid using personal information in passwords</li>
                <li>Consider using a password manager</li>
            </ul>
        </div>
        
        <p>This link will expire in 1 hour for security reasons.</p>
    </div>
    
    <div class="footer">
        <p>© 2024 FLUG Evidence. All rights reserved.</p>
        <p>This email was sent to {to_email}</p>
        <p>If you didn't request a password setup, please ignore this email and contact support.</p>
    </div>
</body>
</html>
        "#);

        let text_content = format!(r#"
Set Your FLUG Evidence Password

Your email has been verified successfully! Now it's time to set up your account password.

Setup URL: {setup_url}

Copy and paste this link into your browser to set your password. This link will expire in 1 hour.

Security Tips:
- Use a strong, unique password
- Include uppercase, lowercase, numbers, and symbols
- Avoid using personal information in passwords
- Consider using a password manager

© 2024 FLUG Evidence. All rights reserved.
This email was sent to {to_email}
If you didn't request a password setup, please ignore this email and contact support.
        "#);

        self.send_email(to_email, subject, &html_content, &text_content).await
    }

    pub async fn send_password_reset_email(&self, to_email: &str, reset_token: &str) -> Result<()> {
        let subject = "Reset Your FLUG Evidence Password";
        let reset_url = format!("http://localhost:8080/reset-password?token={}", reset_token);
        
        let html_content = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Reset Your Password</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .logo {{
            font-size: 28px;
            font-weight: bold;
            color: #E50914;
            margin-bottom: 10px;
        }}
        .card {{
            background: #f8f9fa;
            border-radius: 10px;
            padding: 30px;
            margin: 20px 0;
            border: 1px solid #e9ecef;
        }}
        .button {{
            display: inline-block;
            background: #E50914;
            color: white;
            text-decoration: none;
            padding: 12px 30px;
            border-radius: 5px;
            font-weight: bold;
            margin: 20px 0;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
            text-align: center;
        }}
        .warning {{
            background: #fff3cd;
            border-left: 4px solid #ffc107;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
        .reset-token {{
            font-family: monospace;
            background: #e9ecef;
            padding: 10px;
            border-radius: 5px;
            margin: 10px 0;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">FLUG Evidence</div>
        <div style="color: #6c757d; font-size: 14px;">Decentralized Media Platform</div>
    </div>
    
    <div class="card">
        <h1 style="margin-top: 0;">Reset Your Password</h1>
        
        <p>We received a request to reset your FLUG Evidence account password. If you didn't make this request, you can safely ignore this email.</p>
        
        <div style="text-align: center;">
            <a href="{reset_url}" class="button">Reset Password</a>
        </div>
        
        <p>Or copy and paste this link into your browser:</p>
        <div class="reset-token">{reset_url}</div>
        
        <div class="warning">
            <p><strong>Important:</strong></p>
            <ul>
                <li>This link will expire in 1 hour</li>
                <li>If you didn't request a password reset, your account may be compromised</li>
                <li>Never share your password or this reset link with anyone</li>
            </ul>
        </div>
        
        <p>For security reasons, we recommend using a strong, unique password that you don't use elsewhere.</p>
    </div>
    
    <div class="footer">
        <p>© 2024 FLUG Evidence. All rights reserved.</p>
        <p>This email was sent to {to_email}</p>
        <p>If you didn't request a password reset, please contact our support team immediately.</p>
    </div>
</body>
</html>
        "#);

        let text_content = format!(r#"
Reset Your FLUG Evidence Password

We received a request to reset your FLUG Evidence account password. If you didn't make this request, you can safely ignore this email.

Reset URL: {reset_url}

Copy and paste this link into your browser to reset your password. This link will expire in 1 hour.

Important:
- This link will expire in 1 hour
- If you didn't request a password reset, your account may be compromised
- Never share your password or this reset link with anyone

For security reasons, we recommend using a strong, unique password that you don't use elsewhere.

© 2024 FLUG Evidence. All rights reserved.
This email was sent to {to_email}
If you didn't request a password reset, please contact our support team immediately.
        "#);

        self.send_email(to_email, subject, &html_content, &text_content).await
    }

    pub async fn send_password_changed_notification(&self, to_email: &str) -> Result<()> {
        let subject = "Your FLUG Evidence Password Was Changed";
        
        let html_content = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Password Changed</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .logo {{
            font-size: 28px;
            font-weight: bold;
            color: #E50914;
            margin-bottom: 10px;
        }}
        .card {{
            background: #f8f9fa;
            border-radius: 10px;
            padding: 30px;
            margin: 20px 0;
            border: 1px solid #e9ecef;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
            text-align: center;
        }}
        .warning {{
            background: #fff3cd;
            border-left: 4px solid #ffc107;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
        .success {{
            background: #d4edda;
            border-left: 4px solid #28a745;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">FLUG Evidence</div>
        <div style="color: #6c757d; font-size: 14px;">Decentralized Media Platform</div>
    </div>
    
    <div class="card">
        <h1 style="margin-top: 0;">Password Changed Successfully</h1>
        
        <div class="success">
            <p><strong>✓ Your password has been updated</strong></p>
            <p>Your FLUG Evidence account password was successfully changed.</p>
        </div>
        
        <div class="warning">
            <p><strong>Security Notice:</strong></p>
            <p>If you didn't change your password, please take these steps immediately:</p>
            <ol>
                <li>Use the "Forgot Password" feature to reset your password</li>
                <li>Check your account for any suspicious activity</li>
                <li>Contact our support team if you notice anything unusual</li>
                <li>Review your account security settings</li>
            </ol>
        </div>
        
        <p><strong>Account Security Tips:</strong></p>
        <ul>
            <li>Use a strong, unique password</li>
            <li>Enable two-factor authentication if available</li>
            <li>Regularly review your account activity</li>
            <li>Never share your password with anyone</li>
        </ul>
        
        <p>You can sign in to your account at:</p>
        <p><a href="http://localhost:8080/login">http://localhost:8080/login</a></p>
        
        <p>If you have any questions or concerns, please reply to this email.</p>
    </div>
    
    <div class="footer">
        <p>© 2024 FLUG Evidence. All rights reserved.</p>
        <p>This email was sent to {to_email}</p>
        <p>This is a security notification email. If you believe this was sent in error, please contact support.</p>
    </div>
</body>
</html>
        "#);

        let text_content = format!(r#"
Password Changed Successfully

✓ Your password has been updated

Your FLUG Evidence account password was successfully changed.

Security Notice:
If you didn't change your password, please take these steps immediately:
1. Use the "Forgot Password" feature to reset your password
2. Check your account for any suspicious activity
3. Contact our support team if you notice anything unusual
4. Review your account security settings

Account Security Tips:
- Use a strong, unique password
- Enable two-factor authentication if available
- Regularly review your account activity
- Never share your password with anyone

You can sign in to your account at: http://localhost:8080/login

If you have any questions or concerns, please reply to this email.

© 2024 FLUG Evidence. All rights reserved.
This email was sent to {to_email}
This is a security notification email. If you believe this was sent in error, please contact support.
        "#);

        self.send_email(to_email, subject, &html_content, &text_content).await
    }

    pub async fn send_wallet_connected_notification(&self, to_email: &str, wallet_address: &str, chain: &str) -> Result<()> {
        let subject = "Wallet Connected to FLUG Evidence";
        
        let html_content = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Wallet Connected</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 600px;
            margin: 0 auto;
            padding: 20px;
        }}
        .header {{
            text-align: center;
            margin-bottom: 30px;
        }}
        .logo {{
            font-size: 28px;
            font-weight: bold;
            color: #E50914;
            margin-bottom: 10px;
        }}
        .card {{
            background: #f8f9fa;
            border-radius: 10px;
            padding: 30px;
            margin: 20px 0;
            border: 1px solid #e9ecef;
        }}
        .wallet-info {{
            background: #e8f5e9;
            border-left: 4px solid #4CAF50;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
        .wallet-address {{
            font-family: monospace;
            font-weight: bold;
            word-break: break-all;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e9ecef;
            font-size: 12px;
            color: #6c757d;
            text-align: center;
        }}
        .warning {{
            background: #fff3cd;
            border-left: 4px solid #ffc107;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
        .benefits {{
            background: #e8f4fd;
            border-left: 4px solid #2196F3;
            padding: 15px;
            margin: 20px 0;
            border-radius: 0 5px 5px 0;
        }}
    </style>
</head>
<body>
    <div class="header">
        <div class="logo">FLUG Evidence</div>
        <div style="color: #6c757d; font-size: 14px;">Decentralized Media Platform</div>
    </div>
    
    <div class="card">
        <h1 style="margin-top: 0;">Wallet Connected Successfully</h1>
        
        <p>Your blockchain wallet has been connected to your FLUG Evidence account.</p>
        
        <div class="wallet-info">
            <p><strong>Connected Wallet:</strong></p>
            <div class="wallet-address">{wallet_address}</div>
            <p><strong>Network:</strong> {chain}</p>
            <p><strong>Connected at:</strong> {}</p>
        </div>
        
        <div class="benefits">
            <p><strong>What you can do now:</strong></p>
            <ul>
                <li>Cryptographically sign your uploaded content</li>
                <li>Prove ownership and authenticity of your media</li>
                <li>Login with your wallet instead of email/password</li>
                <li>Create permanent blockchain records of your content</li>
                <li>Verify content integrity on-chain</li>
            </ul>
        </div>
        
        <div class="warning">
            <p><strong>Security Notice:</strong></p>
            <p>If you didn't connect a wallet to your account, or if you notice any suspicious activity, please:</p>
            <ol>
                <li>Immediately disconnect the wallet from your account settings</li>
                <li>Change your account password</li>
                <li>Contact our support team</li>
                <li>Review your account activity</li>
            </ol>
        </div>
        
        <p><strong>Managing Your Wallet:</strong></p>
        <ul>
            <li>You can manage your connected wallets in your account dashboard</li>
            <li>You can disconnect the wallet at any time</li>
            <li>You can connect multiple wallets to your account</li>
        </ul>
        
        <p>If you have any questions about wallet connectivity, please reply to this email.</p>
    </div>
    
    <div class="footer">
        <p>© 2024 FLUG Evidence. All rights reserved.</p>
        <p>This email was sent to {to_email}</p>
        <p>This is a security notification email. If you believe this was sent in error, please contact support.</p>
    </div>
</body>
</html>
        "#, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

        let text_content = format!(r#"
Wallet Connected to FLUG Evidence

Your blockchain wallet has been connected to your FLUG Evidence account.

Connected Wallet: {wallet_address}
Network: {chain}
Connected at: {}

What you can do now:
- Cryptographically sign your uploaded content
- Prove ownership and authenticity of your media
- Login with your wallet instead of email/password
- Create permanent blockchain records of your content
- Verify content integrity on-chain

Security Notice:
If you didn't connect a wallet to your account, or if you notice any suspicious activity, please:
1. Immediately disconnect the wallet from your account settings
2. Change your account password
3. Contact our support team
4. Review your account activity

Managing Your Wallet:
- You can manage your connected wallets in your account dashboard
- You can disconnect the wallet at any time
- You can connect multiple wallets to your account

If you have any questions about wallet connectivity, please reply to this email.

© 2024 FLUG Evidence. All rights reserved.
This email was sent to {to_email}
This is a security notification email. If you believe this was sent in error, please contact support.
        "#, chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));

        self.send_email(to_email, subject, &html_content, &text_content).await
    }

    // Alias for backward compatibility
    pub async fn send_wallet_connected_email(&self, to_email: &str, wallet_address: &str, chain: &str) -> Result<()> {
        self.send_wallet_connected_notification(to_email, wallet_address, chain).await
    }
}