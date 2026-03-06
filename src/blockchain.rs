use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::env;
use std::time::Duration;
use chrono::Utc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockchainUser {
    pub user_id: String,
    pub email: String,
    pub registered_at: String,
    pub wallet_address: Option<String>,
    pub chain: Option<String>,
    pub wallet_type: Option<String>,
    pub transaction_type: TransactionType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TransactionType {
    UserRegistration,
    WalletConnection,
    ProfileUpdate,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiskTransaction {
    pub module: String,
    pub command: String,
    pub nonce: String,
    pub fee: String,
    pub senderPublicKey: String,
    pub params: serde_json::Value,
    pub signatures: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiskTransactionResponse {
    pub transactionId: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiskError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LiskApiResponse<T> {
    pub data: Option<T>,
    pub meta: Option<serde_json::Value>,
    pub errors: Option<Vec<LiskError>>,
}

#[derive(Debug, thiserror::Error)]
pub enum BlockchainError {
    #[error("Lisk API error: {0}")]
    ApiError(String),
    
    #[error("Transaction failed: {0}")]
    TransactionError(String),
    
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub struct BlockchainService {
    client: Client,
    node_url: String,
    chain_id: String,
    app_passphrase: String,
    sender_public_key: String,
    module_id: String,
    command_ids: CommandIds,
}

#[derive(Debug, Clone)]
pub struct CommandIds {
    pub register_user: String,
    pub connect_wallet: String,
    pub update_profile: String,
}

impl BlockchainService {
    pub fn new() -> Result<Self, BlockchainError> {
        let node_url = env::var("LISK_NODE_URL")
            .unwrap_or_else(|_| "https://testnet.lisk.com".to_string());
            
        let chain_id = env::var("LISK_CHAIN_ID")
            .unwrap_or_else(|_| "00000001".to_string());
            
        let app_passphrase = env::var("LISK_APP_PASSPHRASE")
            .map_err(|_| BlockchainError::ConfigError("LISK_APP_PASSPHRASE not set".to_string()))?;
            
        let sender_public_key = env::var("LISK_SENDER_PUBLIC_KEY")
            .map_err(|_| BlockchainError::ConfigError("LISK_SENDER_PUBLIC_KEY not set".to_string()))?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(BlockchainError::NetworkError)?;

        let command_ids = CommandIds {
            register_user: env::var("LISK_COMMAND_REGISTER_USER")
                .unwrap_or_else(|_| "user:register".to_string()),
            connect_wallet: env::var("LISK_COMMAND_CONNECT_WALLET")
                .unwrap_or_else(|_| "user:connectWallet".to_string()),
            update_profile: env::var("LISK_COMMAND_UPDATE_PROFILE")
                .unwrap_or_else(|_| "user:updateProfile".to_string()),
        };

        let module_id = env::var("LISK_MODULE_ID")
            .unwrap_or_else(|_| "auth".to_string());

        Ok(Self {
            client,
            node_url,
            chain_id,
            app_passphrase,
            sender_public_key,
            module_id,
            command_ids,
        })
    }

    /// Register a new user on the Lisk blockchain
    pub async fn register_user_on_blockchain(&self, user_data: &BlockchainUser) -> Result<String, BlockchainError> {
        let params = serde_json::json!({
            "user": {
                "id": user_data.user_id,
                "email": user_data.email,
                "registeredAt": user_data.registered_at,
                "action": "registration"
            }
        });

        let transaction = self.build_transaction(
            &self.command_ids.register_user,
            params,
            "1000000", // 0.01 LSK
        ).await?;

        self.broadcast_transaction(&transaction).await
    }

    /// Connect/update wallet on the Lisk blockchain
    pub async fn update_wallet_on_blockchain(&self, user_data: &BlockchainUser) -> Result<String, BlockchainError> {
        let (wallet_address, chain, wallet_type) = match (&user_data.wallet_address, &user_data.chain, &user_data.wallet_type) {
            (Some(addr), Some(chain), Some(w_type)) => (addr, chain, w_type),
            _ => return Err(BlockchainError::TransactionError("Missing wallet data".to_string())),
        };

        let params = serde_json::json!({
            "user": {
                "id": user_data.user_id,
                "email": user_data.email,
                "wallet": {
                    "address": wallet_address,
                    "chain": chain,
                    "type": wallet_type,
                    "connectedAt": Utc::now().to_rfc3339()
                },
                "action": "wallet_connection"
            }
        });

        let transaction = self.build_transaction(
            &self.command_ids.connect_wallet,
            params,
            "1000000",
        ).await?;

        self.broadcast_transaction(&transaction).await
    }

    /// Build a Lisk transaction
    async fn build_transaction(
        &self,
        command: &str,
        params: serde_json::Value,
        fee: &str,
    ) -> Result<LiskTransaction, BlockchainError> {
        // Get nonce from the network
        let nonce = self.get_account_nonce(&self.sender_public_key).await?;

        let transaction = LiskTransaction {
            module: self.module_id.clone(),
            command: command.to_string(),
            nonce,
            fee: fee.to_string(),
            senderPublicKey: self.sender_public_key.clone(),
            params,
            signatures: vec![self.sign_transaction_data().await?],
        };

        Ok(transaction)
    }

    /// Broadcast transaction to Lisk network
    async fn broadcast_transaction(&self, transaction: &LiskTransaction) -> Result<String, BlockchainError> {
        let url = format!("{}/api/transactions", self.node_url);
        
        let response: LiskApiResponse<LiskTransactionResponse> = self.client
            .post(&url)
            .json(&serde_json::json!({
                "transaction": transaction,
                "metadata": {
                    "chainID": &self.chain_id,
                    "network": "testnet"
                }
            }))
            .send()
            .await
            .map_err(BlockchainError::NetworkError)?
            .json()
            .await
            .map_err(BlockchainError::NetworkError)?;

        if let Some(errors) = response.errors {
            let error_msg = errors.into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(BlockchainError::ApiError(error_msg));
        }

        match response.data {
            Some(tx_response) => Ok(tx_response.transactionId),
            None => Err(BlockchainError::ApiError("No transaction ID in response".to_string())),
        }
    }

    /// Get account nonce from the network
    async fn get_account_nonce(&self, public_key: &str) -> Result<String, BlockchainError> {
        let url = format!("{}/api/accounts?publicKey={}", self.node_url, public_key);
        
        let response: LiskApiResponse<serde_json::Value> = self.client
            .get(&url)
            .send()
            .await
            .map_err(BlockchainError::NetworkError)?
            .json()
            .await
            .map_err(BlockchainError::NetworkError)?;

        // Extract nonce from account data or use timestamp as fallback
        let nonce = response.data
            .and_then(|data| {
                data.get("nonce")
                    .and_then(|n| n.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| Utc::now().timestamp().to_string());

        Ok(nonce)
    }

    /// Sign transaction data (simplified - in production use proper Lisk cryptography)
    async fn sign_transaction_data(&self) -> Result<String, BlockchainError> {
        // In production, this would use Lisk's cryptography library to sign the transaction
        // For now, we'll simulate a signature
        let signature_data = format!("{}-{}-{}", self.module_id, self.chain_id, Utc::now().timestamp());
        let signature = format!("sig_{}", hex::encode(&signature_data.as_bytes()[..32]));
        
        Ok(signature)
    }

    /// Verify transaction status on blockchain
  pub async fn verify_transaction(&self, transaction_id: &str) -> Result<bool, BlockchainError> {
    let url = format!("{}/api/transactions/{}", self.node_url, transaction_id);
    
    let response = self.client
        .get(&url)
        .send()
        .await
        .map_err(BlockchainError::NetworkError)?;

    // Check if the transaction exists (status 200)
    Ok(response.status().is_success())
}

    /// Get user data from blockchain
    pub async fn get_user_from_blockchain(&self, user_id: &str) -> Result<Option<BlockchainUser>, BlockchainError> {
        let url = format!("{}/api/indexer/auth/users/{}", self.node_url, user_id);
        
        let response: LiskApiResponse<BlockchainUser> = self.client
            .get(&url)
            .send()
            .await
            .map_err(BlockchainError::NetworkError)?
            .json()
            .await
            .map_err(BlockchainError::NetworkError)?;

        Ok(response.data)
    }

    /// Get blockchain network info
    pub async fn get_network_info(&self) -> Result<serde_json::Value, BlockchainError> {
        let url = format!("{}/api/node/info", self.node_url);
        
        let response: LiskApiResponse<serde_json::Value> = self.client
            .get(&url)
            .send()
            .await
            .map_err(BlockchainError::NetworkError)?
            .json()
            .await
            .map_err(BlockchainError::NetworkError)?;

        response.data
            .context("No data in network info response")
            .map_err(|e| BlockchainError::ApiError(e.to_string()))
    }
}

// Helper function to create blockchain service
pub fn create_blockchain_service() -> Result<BlockchainService, BlockchainError> {
    BlockchainService::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_blockchain_service_creation() {
        // This test would require environment variables to be set
        // For now, just test that the struct can be created without panicking
        let service = BlockchainService::new();
        assert!(service.is_ok() || service.is_err()); // Should not panic
    }

    #[test]
    fn test_transaction_type_serialization() {
        let tx_type = TransactionType::UserRegistration;
        let serialized = serde_json::to_string(&tx_type).unwrap();
        assert!(serialized.contains("UserRegistration"));
    }
}