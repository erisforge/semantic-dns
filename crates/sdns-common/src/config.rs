use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{PrincipalId, Role, error::AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub http: HttpConfig,
    #[serde(default)]
    pub store: StoreConfig,
    #[serde(default)]
    pub dns: DnsConfig,
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub fathom: FathomConfig,
    #[serde(default = "default_tokens")]
    pub api_tokens: Vec<ApiTokenConfig>,
}

impl AppConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, AppError> {
        if let Some(path) = path {
            return config::Config::builder()
                .add_source(config::File::from(path.to_path_buf()).required(true))
                .build()
                .map_err(|err| AppError::Config(err.to_string()))?
                .try_deserialize()
                .map_err(|err| AppError::Config(err.to_string()));
        }

        Ok(Self {
            http: HttpConfig::default(),
            store: StoreConfig::default(),
            dns: DnsConfig::default(),
            audit: AuditConfig::default(),
            fathom: FathomConfig::default(),
            api_tokens: default_tokens(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    #[serde(default = "default_http_bind")]
    pub bind: String,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            bind: default_http_bind(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    #[serde(default = "default_store_database_url")]
    pub database_url: String,
    #[serde(default = "default_store_schema")]
    pub schema: String,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            database_url: default_store_database_url(),
            schema: default_store_schema(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    #[serde(default = "default_dns_zone")]
    pub zone: String,
    #[serde(default = "default_zone_file")]
    pub zone_file: String,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            zone: default_dns_zone(),
            zone_file: default_zone_file(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    #[serde(default = "default_audit_database_url")]
    pub database_url: String,
    #[serde(default = "default_audit_schema")]
    pub schema: String,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            database_url: default_audit_database_url(),
            schema: default_audit_schema(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FathomConfig {
    #[serde(default)]
    pub database_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiTokenConfig {
    #[serde(default = "PrincipalId::new")]
    pub principal_id: PrincipalId,
    pub name: String,
    pub token: String,
    pub role: Role,
}

fn default_http_bind() -> String {
    "127.0.0.1:8088".to_string()
}

fn default_dns_zone() -> String {
    "local".to_string()
}

fn default_store_database_url() -> String {
    "postgres://postgres:postgres@127.0.0.1:55432/semantic_dns".to_string()
}

fn default_store_schema() -> String {
    "semantic_dns".to_string()
}

fn default_zone_file() -> String {
    "./semantic-dns.zone".to_string()
}

fn default_audit_database_url() -> String {
    default_store_database_url()
}

fn default_audit_schema() -> String {
    default_store_schema()
}

fn default_tokens() -> Vec<ApiTokenConfig> {
    vec![
        ApiTokenConfig {
            principal_id: PrincipalId::new(),
            name: "local-admin".to_string(),
            token: "semantic-admin-token".to_string(),
            role: Role::Admin,
        },
        ApiTokenConfig {
            principal_id: PrincipalId::new(),
            name: "dhcp-engine".to_string(),
            token: "semantic-dhcp-token".to_string(),
            role: Role::System,
        },
    ]
}
