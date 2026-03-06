// src/storj.rs
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::ObjectCannedAcl;
use anyhow::{Result, anyhow};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct StorjService {
    client: S3Client,
    endpoint_url: String,
    bucket_name: String,
    sharing_key: String,  // ADD THIS - store the sharing key
}

#[derive(Debug, Clone)]
pub struct PublicUploadResult {
    pub public_url: String,
    pub bucket: String,
    pub key: String,
    pub etag: String,
}

impl StorjService {
    pub async fn new(
        access_key: &str,
        secret_key: &str,
        endpoint: &str,
        bucket_name: Option<&str>,
        sharing_key: Option<&str>,  // ADD THIS PARAMETER
    ) -> Result<Self> {
        println!("[STORJ] Initializing Storj service");
        println!("[STORJ] Endpoint: {}", endpoint);
        println!("[STORJ] Access key starts with: {}", 
            &access_key[..std::cmp::min(8, access_key.len())]);
        
        let bucket = bucket_name.unwrap_or("crimebank").to_string();
        println!("[STORJ] Bucket: {}", bucket);
        
        // Store the sharing key - if not provided, we'll still work but URLs won't be accessible
        let sharing_key = sharing_key.unwrap_or("").to_string();
        if sharing_key.is_empty() {
            println!("[STORJ] ⚠️ No sharing key provided - public URLs may not work!");
        } else {
            println!("[STORJ] ✅ Sharing key: {}", sharing_key);
        }
        
        // Create credentials
        let credentials = Credentials::new(
            access_key,
            secret_key,
            None,  // session token
            None,  // expires_at
            "storj",
        );

        // Build configuration for S3-compatible storage
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new("us-east-1"))
            .endpoint_url(endpoint)
            .force_path_style(true)
            .build();

        let client = S3Client::from_conf(config);

        let service = Self {
            client,
            endpoint_url: endpoint.to_string(),
            bucket_name: bucket,
            sharing_key,  // Store it
        };

        // Test the connection
        println!("[STORJ] Testing connection...");
        match service.test_connection().await {
            Ok(_) => {
                println!("[STORJ] ✅ Connection successful!");
                
                // Ensure bucket exists
                service.ensure_bucket_exists().await?;
                // Don't try to set bucket policy - it's failing anyway
            }
            Err(e) => {
                println!("[STORJ] ⚠️ Connection test failed: {}", e);
                println!("[STORJ] Will continue but uploads may fail...");
            }
        }

        Ok(service)
    }

    /// Test connection to Storj
    async fn test_connection(&self) -> Result<()> {
        match self.client.list_buckets().send().await {
            Ok(response) => {
                let bucket_count = response.buckets().len();
                println!("[STORJ] Found {} buckets", bucket_count);
                
                // List bucket names if any
                if bucket_count > 0 {
                    println!("[STORJ] Available buckets:");
                    for bucket in response.buckets() {
                        if let Some(name) = bucket.name() {
                            println!("[STORJ]   - {}", name);
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("InvalidAccessKeyId") {
                    Err(anyhow!("Invalid access key ID. Check your Storj credentials."))
                } else if error_msg.contains("SignatureDoesNotMatch") {
                    Err(anyhow!("Invalid secret key. Check your Storj credentials."))
                } else if error_msg.contains("Connection refused") {
                    Err(anyhow!("Connection refused. Check your endpoint URL: {}", self.endpoint_url))
                } else {
                    Err(anyhow!("Connection failed: {}", e))
                }
            }
        }
    }

    /// Simple upload
    pub async fn upload_bytes(
        &self,
        data: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<String> {
        let key = format!("{}-{}", Uuid::new_v4(), filename);
        
        println!("[STORJ] Attempting upload:");
        println!("[STORJ]   Bucket: {}", self.bucket_name);
        println!("[STORJ]   Key: {}", key);
        println!("[STORJ]   Content-Type: {}", content_type);
        println!("[STORJ]   Data size: {} bytes", data.len());
        
        let body = ByteStream::from(data.to_vec());
        
        match self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .content_type(content_type)
            .body(body)
            .send()
            .await
        {
            Ok(_) => {
                let url = self.generate_public_url(&key);  // FIXED: Use generate_public_url
                println!("[STORJ] ✅ Upload SUCCESSFUL!");
                println!("[STORJ] URL: {}", url);
                Ok(url)
            }
            Err(e) => {
                println!("[STORJ] ❌ Upload FAILED: {}", e);
                Err(anyhow!("Storj upload failed: {}", e))
            }
        }
    }

    /// Upload bytes with PUBLIC ACCESS (files will be publicly readable)
    /// Uses country-based directory structure: crimebank/{country}/media/{file}
    pub async fn upload_bytes_with_public_access(
        &self,
        data: &[u8],
        filename: &str,
        content_type: &str,
    ) -> Result<PublicUploadResult> {
        // Default to "Unknown" if no country specified
        self.upload_bytes_with_public_access_country(data, filename, content_type, "Unknown").await
    }

    /// Upload bytes with PUBLIC ACCESS with country specification
    /// Uses: crimebank/{country}/media/{file}
    pub async fn upload_bytes_with_public_access_country(
        &self,
        data: &[u8],
        filename: &str,
        content_type: &str,
        country: &str,
    ) -> Result<PublicUploadResult> {
        // Extract file extension
        let extension = filename
            .rsplit('.')
            .next()
            .unwrap_or("mp4")
            .to_lowercase();
        
        // Normalize country name for directory
        let normalized_country = country.replace(" ", "_").replace("'", "");
        
        // Create a clean filename using only UUID and extension
        let clean_filename = format!("{}.{}", Uuid::new_v4(), extension);
        let key = format!("{}/media/{}", normalized_country, clean_filename);
        
        println!("[STORJ] 📤 Uploading EVIDENCE MEDIA with PUBLIC access:");
        println!("[STORJ]   Country: {}", normalized_country);
        println!("[STORJ]   Original filename: {}", filename);
        println!("[STORJ]   Clean filename: {}", clean_filename);
        println!("[STORJ]   Key: {}", key);
        println!("[STORJ]   Content-Type: {}", content_type);
        println!("[STORJ]   Data size: {} bytes ({:.2} MB)", 
            data.len(), 
            data.len() as f64 / (1024.0 * 1024.0));
        
        let body = ByteStream::from(data.to_vec());
        
        match self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .content_type(content_type)
            .body(body)
            .acl(ObjectCannedAcl::PublicRead)
            .send()
            .await
        {
            Ok(response) => {
                let etag = response.e_tag.unwrap_or_default();
                
                // FIXED: Use link.storjshare.io with sharing key instead of gateway
                let public_url = self.generate_public_url(&key);
                
                println!("[STORJ] ✅ Upload SUCCESSFUL!");
                println!("[STORJ] 🌐 Public URL: {}", public_url);
                println!("[STORJ] 📦 ETag: {}", etag);
                
                // Test the URL asynchronously
                tokio::spawn({
                    let url = public_url.clone();
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        println!("[STORJ] 🔍 Testing uploaded file accessibility...");
                        match reqwest::Client::new().head(&url).send().await {
                            Ok(response) => {
                                println!("[STORJ] ✅ File is accessible! Status: {}", response.status());
                            }
                            Err(e) => {
                                println!("[STORJ] ⚠️ File accessibility test failed: {}", e);
                            }
                        }
                    }
                });
                
                Ok(PublicUploadResult {
                    public_url,
                    bucket: self.bucket_name.clone(),
                    key,
                    etag,
                })
            }
            Err(e) => {
                println!("[STORJ] ❌ Upload FAILED: {}", e);
                Err(anyhow!("Storj upload failed: {}", e))
            }
        }
    }

    /// Upload bytes with PUBLIC ACCESS to TARGET directory (for target photos)
    /// Uses: crimebank/{country}/target/{file}
    pub async fn upload_bytes_with_public_access_target(
        &self,
        data: &[u8],
        filename: &str,
        content_type: &str,
        country: &str,
    ) -> Result<PublicUploadResult> {
        // Extract file extension
        let extension = filename
            .rsplit('.')
            .next()
            .unwrap_or("jpg")
            .to_lowercase();
        
        // Normalize country name for directory
        let normalized_country = country.replace(" ", "_").replace("'", "");
        
        // Create a clean filename using only UUID and extension
        let clean_filename = format!("{}.{}", Uuid::new_v4(), extension);
        let key = format!("{}/target/{}", normalized_country, clean_filename);
        
        println!("[STORJ] 🎯 Uploading TARGET PHOTO with PUBLIC access:");
        println!("[STORJ]   Country: {}", normalized_country);
        println!("[STORJ]   Original filename: {}", filename);
        println!("[STORJ]   Clean filename: {}", clean_filename);
        println!("[STORJ]   Key: {}", key);
        println!("[STORJ]   Content-Type: {}", content_type);
        println!("[STORJ]   Data size: {} bytes ({:.2} MB)", 
            data.len(), 
            data.len() as f64 / (1024.0 * 1024.0));
        
        let body = ByteStream::from(data.to_vec());
        
        match self.client
            .put_object()
            .bucket(&self.bucket_name)
            .key(&key)
            .content_type(content_type)
            .body(body)
            .acl(ObjectCannedAcl::PublicRead)
            .send()
            .await
        {
            Ok(response) => {
                let etag = response.e_tag.unwrap_or_default();
                
                // FIXED: Use link.storjshare.io with sharing key instead of gateway
                let public_url = self.generate_public_url(&key);
                
                println!("[STORJ] ✅ TARGET PHOTO Upload SUCCESSFUL!");
                println!("[STORJ] 🎯 Public URL: {}", public_url);
                println!("[STORJ] 📦 ETag: {}", etag);
                
                // Test the URL asynchronously
                tokio::spawn({
                    let url = public_url.clone();
                    async move {
                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                        println!("[STORJ] 🔍 Testing uploaded target photo accessibility...");
                        match reqwest::Client::new().head(&url).send().await {
                            Ok(response) => {
                                println!("[STORJ] ✅ Target photo is accessible! Status: {}", response.status());
                            }
                            Err(e) => {
                                println!("[STORJ] ⚠️ Target photo accessibility test failed: {}", e);
                            }
                        }
                    }
                });
                
                Ok(PublicUploadResult {
                    public_url,
                    bucket: self.bucket_name.clone(),
                    key,
                    etag,
                })
            }
            Err(e) => {
                println!("[STORJ] ❌ TARGET PHOTO Upload FAILED: {}", e);
                Err(anyhow!("Storj target photo upload failed: {}", e))
            }
        }
    }

    /// REMOVED: ensure_bucket_public - replaced with simpler ensure_bucket_exists
    /// This avoids the failing bucket policy call
    async fn ensure_bucket_exists(&self) -> Result<()> {
        println!("[STORJ] 🔧 Ensuring bucket '{}' exists...", self.bucket_name);
        
        // Check if bucket exists
        match self.client.head_bucket()
            .bucket(&self.bucket_name)
            .send()
            .await {
            Ok(_) => {
                println!("[STORJ] ✅ Bucket '{}' exists", self.bucket_name);
                Ok(())
            }
            Err(e) => {
                println!("[STORJ] 🚧 Bucket doesn't exist, creating...");
                
                // Try to create bucket
                match self.client.create_bucket()
                    .bucket(&self.bucket_name)
                    .send()
                    .await {
                    Ok(_) => {
                        println!("[STORJ] ✅ Created bucket '{}'", self.bucket_name);
                        Ok(())
                    }
                    Err(create_err) => {
                        println!("[STORJ] ❌ Failed to create bucket: {}", create_err);
                        println!("[STORJ] 💡 You may need to create it manually in Storj Dashboard");
                        Err(anyhow!("Failed to create bucket: {}", create_err))
                    }
                }
            }
        }
    }

    /// Test credentials without upload
    pub async fn test_credentials(&self) -> Result<()> {
        println!("[STORJ] Testing credentials...");
        
        match self.client.list_buckets().send().await {
            Ok(response) => {
                let bucket_names: Vec<String> = response.buckets()
                    .iter()
                    .filter_map(|b| b.name().map(|s| s.to_string()))
                    .collect();
                
                println!("[STORJ] ✅ Credentials are VALID!");
                println!("[STORJ] Found {} buckets: {:?}", bucket_names.len(), bucket_names);
                
                if bucket_names.contains(&self.bucket_name) {
                    println!("[STORJ] ✅ Bucket '{}' exists", self.bucket_name);
                } else {
                    println!("[STORJ] ❌ Bucket '{}' does NOT exist", self.bucket_name);
                }
                
                Ok(())
            }
            Err(e) => {
                println!("[STORJ] ❌ Credentials test failed: {}", e);
                Err(anyhow!("Credentials test failed: {}", e))
            }
        }
    }

    /// Create the bucket if it doesn't exist
    pub async fn create_bucket_if_not_exists(&self) -> Result<bool> {
        println!("[STORJ] Checking if bucket exists: {}", self.bucket_name);
        
        let bucket_exists = self.client
            .head_bucket()
            .bucket(&self.bucket_name)
            .send()
            .await
            .is_ok();
        
        if bucket_exists {
            println!("[STORJ] ✅ Bucket '{}' already exists", self.bucket_name);
            return Ok(false);
        }
        
        println!("[STORJ] Creating bucket '{}'...", self.bucket_name);
        
        match self.client
            .create_bucket()
            .bucket(&self.bucket_name)
            .send()
            .await
        {
            Ok(_) => {
                println!("[STORJ] ✅ Successfully created bucket '{}'", self.bucket_name);
                Ok(true)
            }
            Err(e) => {
                println!("[STORJ] ❌ Failed to create bucket: {}", e);
                Err(anyhow!("Failed to create bucket: {}", e))
            }
        }
    }

    /// Helper function to test if a specific file URL is publicly accessible
    pub async fn test_file_accessibility(&self, file_url: &str) -> Result<bool> {
        println!("[STORJ] 🔍 Testing file accessibility: {}", file_url);
        
        match reqwest::get(file_url).await {
            Ok(response) => {
                let status = response.status();
                println!("[STORJ] Response status: {}", status);
                
                if status.is_success() {
                    println!("[STORJ] ✅ File is publicly accessible!");
                    Ok(true)
                } else {
                    println!("[STORJ] ❌ File is NOT accessible. Status: {}", status);
                    Ok(false)
                }
            }
            Err(e) => {
                println!("[STORJ] ❌ Failed to test file accessibility: {}", e);
                Ok(false)
            }
        }
    }

    /// FIXED: Generate a public URL using link.storjshare.io with sharing key
    pub fn generate_public_url(&self, key: &str) -> String {
        if self.sharing_key.is_empty() {
            // Fallback to gateway URL if no sharing key (won't work without public ACL)
            format!("https://gateway.storjshare.io/{}/{}", self.bucket_name, key)
        } else {
            // CORRECT: Use link.storjshare.io with sharing key for public access
            format!(
                "https://link.storjshare.io/raw/{}/{}/{}",
                self.sharing_key, self.bucket_name, key
            )
        }
    }

    /// Get the bucket name
    pub fn bucket_name(&self) -> &str {
        &self.bucket_name
    }

    /// Get the endpoint URL
    pub fn endpoint_url(&self) -> &str {
        &self.endpoint_url
    }

    /// Get the base public URL for the bucket using sharing key
    pub fn base_public_url(&self) -> String {
        if self.sharing_key.is_empty() {
            format!("https://gateway.storjshare.io/{}", self.bucket_name)
        } else {
            format!(
                "https://link.storjshare.io/raw/{}/{}",
                self.sharing_key, self.bucket_name
            )
        }
    }

    /// List all objects in the bucket (for debugging)
    pub async fn list_objects(&self) -> Result<Vec<String>> {
        println!("[STORJ] Listing objects in bucket '{}'", self.bucket_name);
        
        match self.client
            .list_objects_v2()
            .bucket(&self.bucket_name)
            .send()
            .await
        {
            Ok(response) => {
                let objects: Vec<String> = response.contents()
                    .iter()
                    .filter_map(|obj| obj.key().map(|s| s.to_string()))
                    .collect();
                
                println!("[STORJ] Found {} objects", objects.len());
                for (i, obj) in objects.iter().enumerate() {
                    println!("[STORJ]   {}. {}", i + 1, obj);
                }
                
                Ok(objects)
            }
            Err(e) => {
                println!("[STORJ] ❌ Failed to list objects: {}", e);
                Err(anyhow!("Failed to list objects: {}", e))
            }
        }
    }

    /// Initialize country directory structure for all countries
    /// Creates: crimebank/{country}/media/ and crimebank/{country}/target/
    pub async fn initialize_country_directories(&self, countries: &[crate::countries::Country]) -> Result<()> {
        println!("[STORJ] 🌍 Initializing country directory structure...");
        println!("[STORJ] Total countries to initialize: {}", countries.len());
        
        let mut initialized = 0;
        let mut failed = 0;
        
        for country in countries {
            let normalized_name = country.name.replace(" ", "_").replace("'", "");
            
            // Create dummy file in media directory
            let media_key = format!("{}/media/.init", normalized_name);
            // Create dummy file in target directory  
            let target_key = format!("{}/target/.init", normalized_name);
            
            // Upload tiny placeholder files to create directory structure
            let placeholder = b"init";
            
            // Create media directory
            match self.client
                .put_object()
                .bucket(&self.bucket_name)
                .key(&media_key)
                .body(ByteStream::from(placeholder.to_vec()))
                .send()
                .await
            {
                Ok(_) => {
                    // Create target directory
                    match self.client
                        .put_object()
                        .bucket(&self.bucket_name)
                        .key(&target_key)
                        .body(ByteStream::from(placeholder.to_vec()))
                        .send()
                        .await
                    {
                        Ok(_) => {
                            initialized += 1;
                            if initialized % 20 == 0 {
                                println!("[STORJ] ✅ Initialized {}/{} countries...", initialized, countries.len());
                            }
                        }
                        Err(_) => {
                            failed += 1;
                        }
                    }
                }
                Err(_) => {
                    failed += 1;
                }
            }
        }
        
        println!("[STORJ] 🌍 Country initialization complete!");
        println!("[STORJ] ✅ Successfully initialized: {}", initialized);
        if failed > 0 {
            println!("[STORJ] ⚠️ Failed to initialize: {}", failed);
        }
        
        Ok(())
    }

    /// Delete an object from the bucket
    pub async fn delete_object(&self, key: &str) -> Result<()> {
        println!("[STORJ] Deleting object: {}", key);
        
        match self.client
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
        {
            Ok(_) => {
                println!("[STORJ] ✅ Successfully deleted object: {}", key);
                Ok(())
            }
            Err(e) => {
                println!("[STORJ] ❌ Failed to delete object: {}", e);
                Err(anyhow!("Failed to delete object: {}", e))
            }
        }
    }

    /// Ensure the encodings bucket exists — same pattern as ensure_bucket_exists()
    /// but targets the dedicated "encodings" bucket instead of self.bucket_name.
    pub async fn ensure_encodings_bucket_exists(&self) -> Result<()> {
        let bucket = "encodings";
        println!("[STORJ] 🔧 Ensuring bucket '{}' exists...", bucket);

        match self.client
            .head_bucket()
            .bucket(bucket)
            .send()
            .await
        {
            Ok(_) => {
                println!("[STORJ] ✅ Bucket '{}' exists", bucket);
                Ok(())
            }
            Err(_) => {
                println!("[STORJ] 🚧 Bucket '{}' not found — creating...", bucket);
                match self.client
                    .create_bucket()
                    .bucket(bucket)
                    .send()
                    .await
                {
                    Ok(_) => {
                        println!("[STORJ] ✅ Created bucket '{}'", bucket);
                        Ok(())
                    }
                    Err(create_err) => {
                        println!("[STORJ] ❌ Failed to create bucket '{}': {}", bucket, create_err);
                        println!("[STORJ] 💡 You may need to create it manually in the Storj Dashboard");
                        Err(anyhow!("Failed to create encodings bucket: {}", create_err))
                    }
                }
            }
        }
    }

    /// Upload face encodings pickle to the dedicated "encodings" bucket.
    /// Key: encodings.pickle — fixed, always overwrites the previous version.
    /// Bucket is auto-created if it doesn't exist (same pattern as crimebank).
    /// Public URL is the stable endpoint your Python consumer network downloads from.
    pub async fn upload_encoding_file(&self, data: &[u8]) -> Result<String> {
        let bucket = "encodings";
        let key    = "encodings.pickle".to_string();

        println!("[STORJ] 🥒 Uploading FACE ENCODINGS PICKLE:");
        println!("[STORJ]   Bucket: {}", bucket);
        println!("[STORJ]   Key: {}", key);
        println!("[STORJ]   Size: {} bytes ({:.2} MB)",
            data.len(), data.len() as f64 / (1024.0 * 1024.0));

        // Ensure the dedicated encodings bucket exists before uploading
        self.ensure_encodings_bucket_exists().await?;

        let body = ByteStream::from(data.to_vec());

        match self.client
            .put_object()
            .bucket(bucket)
            .key(&key)
            .content_type("application/octet-stream")
            .body(body)
            .acl(ObjectCannedAcl::PublicRead)
            .send()
            .await
        {
            Ok(response) => {
                let etag = response.e_tag.unwrap_or_default();
                // Build public URL pointing at the encodings bucket
                let public_url = format!(
                    "https://link.storjshare.io/raw/{}/{}/{}",
                    self.sharing_key, bucket, key
                );
                println!("[STORJ] ✅ ENCODINGS PICKLE uploaded successfully!");
                println!("[STORJ] 🥒 Public URL: {}", public_url);
                println!("[STORJ] 📦 ETag: {}", etag);
                Ok(public_url)
            }
            Err(e) => {
                println!("[STORJ] ❌ ENCODINGS PICKLE upload FAILED: {}", e);
                Err(anyhow!("Storj encodings upload failed: {}", e))
            }
        }
    }
}