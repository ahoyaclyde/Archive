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

    async fn send_email(&self, to_email: &str, subject: &str, html_content: &str, text_content: &str) -> Result<()> {
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

    pub async fn send_wallet_connected_email(&self, to_email: &str, wallet_address: &str, chain: &str) -> Result<()> {
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
        </div>
        
        <p><strong>What you can do now:</strong></p>
        <ul>
            <li>Cryptographically sign your uploaded content</li>
            <li>Prove ownership and authenticity of your media</li>
            <li>Login with your wallet instead of email/password</li>
            <li>Create permanent blockchain records of your content</li>
        </ul>
        
        <div class="warning">
            <p><strong>Security Notice:</strong></p>
            <p>If you didn't connect a wallet to your account, or if you notice any suspicious activity, please:</p>
            <ol>
                <li>Immediately disconnect the wallet from your account settings</li>
                <li>Change your account password</li>
                <li>Contact our support team</li>
            </ol>
        </div>
        
        <p>You can manage your connected wallets in your account dashboard.</p>
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
Wallet Connected to FLUG Evidence

Your blockchain wallet has been connected to your FLUG Evidence account.

Connected Wallet: {wallet_address}
Network: {chain}

What you can do now:
- Cryptographically sign your uploaded content
- Prove ownership and authenticity of your media
- Login with your wallet instead of email/password
- Create permanent blockchain records of your content

Security Notice:
If you didn't connect a wallet to your account, or if you notice any suspicious activity, please:
1. Immediately disconnect the wallet from your account settings
2. Change your account password
3. Contact our support team

You can manage your connected wallets in your account dashboard.

© 2024 FLUG Evidence. All rights reserved.
This email was sent to {to_email}
This is a security notification email. If you believe this was sent in error, please contact support.
        "#);

        self.send_email(to_email, subject, &html_content, &text_content).await
    }
}