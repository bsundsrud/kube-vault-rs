use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct VaultResponse<D> {
    pub request_id: String,
    pub lease_id: String,
    pub renewable: bool,
    pub lease_duration: i64,
    pub data: Option<D>,
    pub wrap_info: Option<Value>,
    pub warnings: Option<Value>,
    pub auth: Option<AuthInfo>,
}

#[derive(Debug, Deserialize)]
pub struct AuthInfo {
    pub client_token: String,
    pub accessor: String,
    pub policies: Vec<String>,
    pub token_policies: Vec<String>,
    pub metadata: Value,
    pub lease_duration: Option<i64>,
    pub renewable: bool,
    pub entity_id: String,
    pub token_type: String,
    pub orphan: bool,
}

#[derive(Debug, Deserialize)]
pub struct KvMetadata {
    pub created_time: String,
    pub deletion_time: Option<String>,
    pub destroyed: bool,
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct KvData {
    pub data: HashMap<String, String>,
    pub metadata: KvMetadata,
}

#[derive(Debug, Deserialize)]
pub struct KvKeys {
    pub keys: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct VaultError {
    pub errors: Vec<String>,
}
