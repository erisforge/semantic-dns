use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use async_trait::async_trait;
use chrono::Utc;
use sdns_common::{AppError, DeviceId, FingerprintId, TemplateId};
use sdns_core::{
    HardwareIdentity, HardwareIdentityKind, MetadataField, Observation, ObservationSource,
    RecordFilter, SemanticRecord, SyncStatus, build_semantic_name, merge_observation,
};
use sdns_dhcp::{
    AuthorizeQuarantineRequest, DhcpLease, FingerprintRule, QuarantineEntry, RoleTemplate,
};
use sqlx_core::{query::query, row::Row, types::Json};
use sqlx_postgres::{PgPool, PgPoolOptions, PgRow};
use tokio::sync::RwLock;

#[async_trait]
pub trait SemanticStore: Send + Sync {
    async fn upsert_observation(
        &self,
        observation: Observation,
    ) -> Result<SemanticRecord, AppError>;
    async fn get_record(&self, device_id: DeviceId) -> Result<Option<SemanticRecord>, AppError>;
    async fn resolve(&self, target: &str) -> Result<Option<SemanticRecord>, AppError>;
    async fn query(&self, filter: RecordFilter) -> Result<Vec<SemanticRecord>, AppError>;
    async fn list_leases(&self) -> Result<Vec<DhcpLease>, AppError>;
    async fn upsert_lease(&self, lease: DhcpLease) -> Result<(), AppError>;
    async fn list_quarantine(&self) -> Result<Vec<QuarantineEntry>, AppError>;
    async fn enqueue_quarantine(&self, entry: QuarantineEntry) -> Result<(), AppError>;
    async fn authorize_quarantine(
        &self,
        request: AuthorizeQuarantineRequest,
    ) -> Result<Option<QuarantineEntry>, AppError>;
    async fn sync_status(&self) -> Result<SyncStatus, AppError>;
    async fn mark_reconciliation(&self) -> Result<SyncStatus, AppError>;
    async fn list_fingerprints(&self) -> Result<Vec<FingerprintRule>, AppError>;
    async fn put_fingerprint(&self, rule: FingerprintRule) -> Result<(), AppError>;
    async fn list_templates(&self) -> Result<Vec<RoleTemplate>, AppError>;
    async fn put_template(&self, template: RoleTemplate) -> Result<(), AppError>;
}

#[derive(Default)]
struct StoreState {
    records: BTreeMap<DeviceId, SemanticRecord>,
    leases: HashMap<String, DhcpLease>,
    quarantine: HashMap<String, QuarantineEntry>,
    fingerprints: BTreeMap<FingerprintId, FingerprintRule>,
    templates: BTreeMap<TemplateId, RoleTemplate>,
    sync_status: SyncStatus,
}

#[derive(Clone, Default)]
pub struct InMemoryStore {
    state: Arc<RwLock<StoreState>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SemanticStore for InMemoryStore {
    async fn upsert_observation(
        &self,
        observation: Observation,
    ) -> Result<SemanticRecord, AppError> {
        let mut state = self.state.write().await;
        let existing = state.records.get(&observation.device_id).cloned();
        let sibling_names = state
            .records
            .values()
            .filter(|record| record.device_id != observation.device_id)
            .map(|record| record.fqdn.clone())
            .collect::<Vec<_>>();
        let record = merge_observation(existing, &observation, &sibling_names, "local")?;
        if matches!(observation.source, ObservationSource::ManualApi) {
            validate_parent_chain(&record, &sibling_names, "local")?;
        }
        state.records.insert(observation.device_id, record.clone());
        state.sync_status.dns_records_synced = state.records.len();
        Ok(record)
    }

    async fn get_record(&self, device_id: DeviceId) -> Result<Option<SemanticRecord>, AppError> {
        Ok(self.state.read().await.records.get(&device_id).cloned())
    }

    async fn resolve(&self, target: &str) -> Result<Option<SemanticRecord>, AppError> {
        let state = self.state.read().await;
        Ok(state
            .records
            .values()
            .find(|record| {
                record.fqdn.eq_ignore_ascii_case(target)
                    || record.external_ip.as_deref() == Some(target)
                    || record.internal_ip.as_deref() == Some(target)
                    || record.has_hardware_identity(target)
                    || record.has_alias(target)
                    || record.has_application_identity(target)
            })
            .cloned())
    }

    async fn query(&self, filter: RecordFilter) -> Result<Vec<SemanticRecord>, AppError> {
        Ok(filter_records(
            self.state
                .read()
                .await
                .records
                .values()
                .cloned()
                .collect::<Vec<_>>(),
            filter,
        ))
    }

    async fn list_leases(&self) -> Result<Vec<DhcpLease>, AppError> {
        Ok(self.state.read().await.leases.values().cloned().collect())
    }

    async fn upsert_lease(&self, lease: DhcpLease) -> Result<(), AppError> {
        let mut state = self.state.write().await;
        state.leases.insert(lease.mac.clone(), lease.clone());
        if let Some(record) = state.records.values_mut().find(|record| lease_matches_record(record, &lease)) {
            hydrate_record_from_lease(record, &lease);
        }
        state.sync_status.total_leases = state.leases.len();
        state.sync_status.pending_updates = state.sync_status.pending_updates.saturating_add(1);
        Ok(())
    }

    async fn list_quarantine(&self) -> Result<Vec<QuarantineEntry>, AppError> {
        Ok(self
            .state
            .read()
            .await
            .quarantine
            .values()
            .cloned()
            .collect())
    }

    async fn enqueue_quarantine(&self, entry: QuarantineEntry) -> Result<(), AppError> {
        self.state
            .write()
            .await
            .quarantine
            .insert(entry.mac.clone(), entry);
        Ok(())
    }

    async fn authorize_quarantine(
        &self,
        request: AuthorizeQuarantineRequest,
    ) -> Result<Option<QuarantineEntry>, AppError> {
        let removed = self.state.write().await.quarantine.remove(&request.mac);
        Ok(removed)
    }

    async fn sync_status(&self) -> Result<SyncStatus, AppError> {
        Ok(self.state.read().await.sync_status.clone())
    }

    async fn mark_reconciliation(&self) -> Result<SyncStatus, AppError> {
        let mut state = self.state.write().await;
        state.sync_status.pending_updates = 0;
        state.sync_status.last_reconciliation = Some(Utc::now());
        Ok(state.sync_status.clone())
    }

    async fn list_fingerprints(&self) -> Result<Vec<FingerprintRule>, AppError> {
        Ok(self
            .state
            .read()
            .await
            .fingerprints
            .values()
            .cloned()
            .collect())
    }

    async fn put_fingerprint(&self, rule: FingerprintRule) -> Result<(), AppError> {
        self.state.write().await.fingerprints.insert(rule.id, rule);
        Ok(())
    }

    async fn list_templates(&self) -> Result<Vec<RoleTemplate>, AppError> {
        Ok(self
            .state
            .read()
            .await
            .templates
            .values()
            .cloned()
            .collect())
    }

    async fn put_template(&self, template: RoleTemplate) -> Result<(), AppError> {
        self.state
            .write()
            .await
            .templates
            .insert(template.id, template);
        Ok(())
    }
}

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
    schema: String,
}

impl PostgresStore {
    pub async fn open(database_url: &str, schema: &str) -> Result<Self, AppError> {
        let schema = validate_schema_name(schema)?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|err| AppError::Internal(format!("postgres open failed: {err}")))?;
        let store = Self { pool, schema };
        store.run_migrations().await?;
        Ok(store)
    }

    fn table(&self, name: &str) -> String {
        format!("{}.{}", self.schema, name)
    }

    async fn run_migrations(&self) -> Result<(), AppError> {
        query(&format!("CREATE SCHEMA IF NOT EXISTS {}", self.schema))
            .execute(&self.pool)
            .await
            .map_err(|err| AppError::Internal(format!("create schema failed: {err}")))?;

        for statement in [
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    device_id TEXT PRIMARY KEY,
                    fqdn TEXT NOT NULL,
                    external_ip TEXT,
                    internal_ip TEXT,
                    record_json JSONB NOT NULL,
                    updated_at TIMESTAMPTZ NOT NULL
                )",
                self.table("semantic_records")
            ),
            format!(
                "CREATE INDEX IF NOT EXISTS idx_semantic_records_fqdn ON {} (fqdn)",
                self.table("semantic_records")
            ),
            format!(
                "CREATE INDEX IF NOT EXISTS idx_semantic_records_external_ip ON {} (external_ip)",
                self.table("semantic_records")
            ),
            format!(
                "CREATE INDEX IF NOT EXISTS idx_semantic_records_internal_ip ON {} (internal_ip)",
                self.table("semantic_records")
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    mac TEXT PRIMARY KEY,
                    lease_json JSONB NOT NULL,
                    updated_at TIMESTAMPTZ NOT NULL
                )",
                self.table("dhcp_leases")
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    mac TEXT PRIMARY KEY,
                    entry_json JSONB NOT NULL,
                    updated_at TIMESTAMPTZ NOT NULL
                )",
                self.table("quarantine_entries")
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    id TEXT PRIMARY KEY,
                    rule_json JSONB NOT NULL
                )",
                self.table("fingerprint_rules")
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    id TEXT PRIMARY KEY,
                    template_json JSONB NOT NULL
                )",
                self.table("role_templates")
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    id SMALLINT PRIMARY KEY CHECK (id = 1),
                    state_json JSONB NOT NULL
                )",
                self.table("sync_state")
            ),
        ] {
            query(&statement)
                .execute(&self.pool)
                .await
                .map_err(|err| AppError::Internal(format!("postgres migration failed: {err}")))?;
        }

        if self.load_sync_status().await?.is_none() {
            self.save_sync_status(&SyncStatus::default()).await?;
        }

        Ok(())
    }

    async fn list_records(&self) -> Result<Vec<SemanticRecord>, AppError> {
        let rows = query(&format!(
            "SELECT record_json FROM {} ORDER BY updated_at DESC",
            self.table("semantic_records")
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("list records failed: {err}")))?;

        rows.into_iter()
            .map(|row| parse_json_column::<SemanticRecord>(&row, "record_json", "semantic record"))
            .collect()
    }

    async fn save_record(&self, record: &SemanticRecord) -> Result<(), AppError> {
        query(&format!(
            "INSERT INTO {} (device_id, fqdn, external_ip, internal_ip, record_json, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (device_id) DO UPDATE SET
                fqdn = EXCLUDED.fqdn,
                external_ip = EXCLUDED.external_ip,
                internal_ip = EXCLUDED.internal_ip,
                record_json = EXCLUDED.record_json,
                updated_at = EXCLUDED.updated_at",
            self.table("semantic_records")
        ))
        .bind(record.device_id.to_string())
        .bind(&record.fqdn)
        .bind(&record.external_ip)
        .bind(&record.internal_ip)
        .bind(Json(record.clone()))
        .bind(record.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("save record failed: {err}")))?;
        Ok(())
    }

    async fn save_sync_status(&self, status: &SyncStatus) -> Result<(), AppError> {
        query(&format!(
            "INSERT INTO {} (id, state_json) VALUES (1, $1)
             ON CONFLICT (id) DO UPDATE SET state_json = EXCLUDED.state_json",
            self.table("sync_state")
        ))
        .bind(Json(status.clone()))
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("save sync status failed: {err}")))?;
        Ok(())
    }

    async fn load_sync_status(&self) -> Result<Option<SyncStatus>, AppError> {
        let row = query(&format!(
            "SELECT state_json FROM {} WHERE id = 1",
            self.table("sync_state")
        ))
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("load sync status failed: {err}")))?;

        row.map(|row| parse_json_column::<SyncStatus>(&row, "state_json", "sync status"))
            .transpose()
    }
}

#[async_trait]
impl SemanticStore for PostgresStore {
    async fn upsert_observation(
        &self,
        observation: Observation,
    ) -> Result<SemanticRecord, AppError> {
        let existing = self.get_record(observation.device_id).await?;
        let sibling_names = query(&format!(
            "SELECT fqdn FROM {} WHERE device_id != $1",
            self.table("semantic_records")
        ))
        .bind(observation.device_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("load siblings failed: {err}")))?
        .into_iter()
        .map(|row| row.get::<String, _>("fqdn"))
        .collect::<Vec<_>>();

        let record = merge_observation(existing, &observation, &sibling_names, "local")?;
        if matches!(observation.source, ObservationSource::ManualApi) {
            validate_parent_chain(&record, &sibling_names, "local")?;
        }
        self.save_record(&record).await?;

        let mut status = self.load_sync_status().await?.unwrap_or_default();
        status.dns_records_synced = self.list_records().await?.len();
        self.save_sync_status(&status).await?;
        Ok(record)
    }

    async fn get_record(&self, device_id: DeviceId) -> Result<Option<SemanticRecord>, AppError> {
        let row = query(&format!(
            "SELECT record_json FROM {} WHERE device_id = $1",
            self.table("semantic_records")
        ))
        .bind(device_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("get record failed: {err}")))?;

        row.map(|row| parse_json_column::<SemanticRecord>(&row, "record_json", "semantic record"))
            .transpose()
    }

    async fn resolve(&self, target: &str) -> Result<Option<SemanticRecord>, AppError> {
        let records = self.list_records().await?;
        Ok(records.into_iter().find(|record| {
            record.fqdn.eq_ignore_ascii_case(target)
                || record.external_ip.as_deref() == Some(target)
                || record.internal_ip.as_deref() == Some(target)
                || record.has_hardware_identity(target)
                || record.has_alias(target)
                || record.has_application_identity(target)
        }))
    }

    async fn query(&self, filter: RecordFilter) -> Result<Vec<SemanticRecord>, AppError> {
        Ok(filter_records(self.list_records().await?, filter))
    }

    async fn list_leases(&self) -> Result<Vec<DhcpLease>, AppError> {
        let rows = query(&format!(
            "SELECT lease_json FROM {} ORDER BY updated_at DESC",
            self.table("dhcp_leases")
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("list leases failed: {err}")))?;

        rows.into_iter()
            .map(|row| parse_json_column::<DhcpLease>(&row, "lease_json", "lease"))
            .collect()
    }

    async fn upsert_lease(&self, lease: DhcpLease) -> Result<(), AppError> {
        query(&format!(
            "INSERT INTO {} (mac, lease_json, updated_at) VALUES ($1, $2, $3)
             ON CONFLICT (mac) DO UPDATE SET
                lease_json = EXCLUDED.lease_json,
                updated_at = EXCLUDED.updated_at",
            self.table("dhcp_leases")
        ))
        .bind(&lease.mac)
        .bind(Json(lease.clone()))
        .bind(lease.last_seen)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("save lease failed: {err}")))?;

        if let Some(mut record) = self
            .list_records()
            .await?
            .into_iter()
            .find(|record| lease_matches_record(record, &lease))
        {
            hydrate_record_from_lease(&mut record, &lease);
            self.save_record(&record).await?;
        }

        let mut status = self.load_sync_status().await?.unwrap_or_default();
        status.total_leases = self.list_leases().await?.len();
        status.pending_updates = status.pending_updates.saturating_add(1);
        self.save_sync_status(&status).await?;
        Ok(())
    }

    async fn list_quarantine(&self) -> Result<Vec<QuarantineEntry>, AppError> {
        let rows = query(&format!(
            "SELECT entry_json FROM {} ORDER BY updated_at DESC",
            self.table("quarantine_entries")
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("list quarantine failed: {err}")))?;

        rows.into_iter()
            .map(|row| parse_json_column::<QuarantineEntry>(&row, "entry_json", "quarantine entry"))
            .collect()
    }

    async fn enqueue_quarantine(&self, entry: QuarantineEntry) -> Result<(), AppError> {
        query(&format!(
            "INSERT INTO {} (mac, entry_json, updated_at) VALUES ($1, $2, $3)
             ON CONFLICT (mac) DO UPDATE SET
                entry_json = EXCLUDED.entry_json,
                updated_at = EXCLUDED.updated_at",
            self.table("quarantine_entries")
        ))
        .bind(&entry.mac)
        .bind(Json(entry.clone()))
        .bind(entry.queued_at)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("save quarantine entry failed: {err}")))?;
        Ok(())
    }

    async fn authorize_quarantine(
        &self,
        request: AuthorizeQuarantineRequest,
    ) -> Result<Option<QuarantineEntry>, AppError> {
        let existing = query(&format!(
            "SELECT entry_json FROM {} WHERE mac = $1",
            self.table("quarantine_entries")
        ))
        .bind(&request.mac)
        .fetch_optional(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("load quarantine entry failed: {err}")))?;

        query(&format!(
            "DELETE FROM {} WHERE mac = $1",
            self.table("quarantine_entries")
        ))
        .bind(&request.mac)
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("delete quarantine entry failed: {err}")))?;

        existing
            .map(|row| parse_json_column::<QuarantineEntry>(&row, "entry_json", "quarantine entry"))
            .transpose()
    }

    async fn sync_status(&self) -> Result<SyncStatus, AppError> {
        Ok(self.load_sync_status().await?.unwrap_or_default())
    }

    async fn mark_reconciliation(&self) -> Result<SyncStatus, AppError> {
        let mut status = self.load_sync_status().await?.unwrap_or_default();
        status.pending_updates = 0;
        status.last_reconciliation = Some(Utc::now());
        self.save_sync_status(&status).await?;
        Ok(status)
    }

    async fn list_fingerprints(&self) -> Result<Vec<FingerprintRule>, AppError> {
        let rows = query(&format!(
            "SELECT rule_json FROM {} ORDER BY id",
            self.table("fingerprint_rules")
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("list fingerprints failed: {err}")))?;

        rows.into_iter()
            .map(|row| parse_json_column::<FingerprintRule>(&row, "rule_json", "fingerprint rule"))
            .collect()
    }

    async fn put_fingerprint(&self, rule: FingerprintRule) -> Result<(), AppError> {
        query(&format!(
            "INSERT INTO {} (id, rule_json) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET rule_json = EXCLUDED.rule_json",
            self.table("fingerprint_rules")
        ))
        .bind(rule.id.to_string())
        .bind(Json(rule))
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("save fingerprint failed: {err}")))?;
        Ok(())
    }

    async fn list_templates(&self) -> Result<Vec<RoleTemplate>, AppError> {
        let rows = query(&format!(
            "SELECT template_json FROM {} ORDER BY id",
            self.table("role_templates")
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("list templates failed: {err}")))?;

        rows.into_iter()
            .map(|row| parse_json_column::<RoleTemplate>(&row, "template_json", "role template"))
            .collect()
    }

    async fn put_template(&self, template: RoleTemplate) -> Result<(), AppError> {
        query(&format!(
            "INSERT INTO {} (id, template_json) VALUES ($1, $2)
             ON CONFLICT (id) DO UPDATE SET template_json = EXCLUDED.template_json",
            self.table("role_templates")
        ))
        .bind(template.id.to_string())
        .bind(Json(template))
        .execute(&self.pool)
        .await
        .map_err(|err| AppError::Internal(format!("save template failed: {err}")))?;
        Ok(())
    }
}

fn parse_json_column<T>(row: &PgRow, column: &str, label: &str) -> Result<T, AppError>
where
    T: serde::de::DeserializeOwned,
{
    row.try_get::<Json<T>, _>(column)
        .map(|value| value.0)
        .map_err(|err| AppError::Internal(format!("failed to parse {label}: {err}")))
}

fn validate_schema_name(schema: &str) -> Result<String, AppError> {
    if schema.is_empty()
        || !schema
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(AppError::Config(format!(
            "invalid postgres schema `{schema}`; use lowercase letters, digits, or underscores"
        )));
    }

    Ok(schema.to_string())
}

fn filter_records(records: Vec<SemanticRecord>, filter: RecordFilter) -> Vec<SemanticRecord> {
    let q = filter.q.as_ref().map(|value| value.to_ascii_lowercase());
    records
        .into_iter()
        .filter(|record| {
            filter
                .node_kind
                .as_ref()
                .map(|value| &record.node_kind == value)
                .unwrap_or(true)
                && filter
                .hardware_identity
                .as_ref()
                .map(|value| record.has_hardware_identity(value))
                .unwrap_or(true)
                && filter
                .application_id
                .as_ref()
                .map(|value| record.has_application_identity(value))
                .unwrap_or(true)
                && filter
                .alias
                .as_ref()
                .map(|value| record.has_alias(value))
                .unwrap_or(true)
                && filter
                .class
                .as_ref()
                .map(|value| record.class.as_deref() == Some(value.as_str()))
                .unwrap_or(true)
                && filter
                    .vendor
                    .as_ref()
                    .map(|value| record.vendor.as_deref() == Some(value.as_str()))
                    .unwrap_or(true)
                && filter
                    .enterprise
                    .as_ref()
                    .map(|value| record.enterprise.as_deref() == Some(value.as_str()))
                    .unwrap_or(true)
                && filter
                    .site
                    .as_ref()
                    .map(|value| record.effective_site() == Some(value.as_str()))
                    .unwrap_or(true)
                && filter
                    .area
                    .as_ref()
                    .map(|value| record.effective_area() == Some(value.as_str()))
                    .unwrap_or(true)
                && filter
                    .work_center
                    .as_ref()
                    .map(|value| record.effective_work_center() == Some(value.as_str()))
                    .unwrap_or(true)
                && filter
                    .work_center_kind
                    .as_ref()
                    .map(|value| record.work_center_kind.as_ref() == Some(value))
                    .unwrap_or(true)
                && filter
                    .work_unit
                    .as_ref()
                    .map(|value| record.effective_work_unit() == Some(value.as_str()))
                    .unwrap_or(true)
                && filter
                    .cell
                    .as_ref()
                    .map(|value| record.effective_work_center() == Some(value.as_str()))
                    .unwrap_or(true)
                && filter
                    .zone
                    .as_ref()
                    .map(|value| record.effective_area() == Some(value.as_str()))
                    .unwrap_or(true)
                && q.as_ref()
                    .map(|needle| {
                        record.fqdn.to_ascii_lowercase().contains(needle)
                            || record
                                .effective_site()
                                .map(|value| value.to_ascii_lowercase().contains(needle))
                                .unwrap_or(false)
                            || record
                                .effective_area()
                                .map(|value| value.to_ascii_lowercase().contains(needle))
                                .unwrap_or(false)
                            || record
                                .effective_work_center()
                                .map(|value| value.to_ascii_lowercase().contains(needle))
                                .unwrap_or(false)
                            || record
                                .effective_work_unit()
                                .map(|value| value.to_ascii_lowercase().contains(needle))
                                .unwrap_or(false)
                            || record
                                .effective_leaf()
                                .map(|value| value.to_ascii_lowercase().contains(needle))
                                .unwrap_or(false)
                            || record
                                .model
                                .as_deref()
                                .map(|value| value.to_ascii_lowercase().contains(needle))
                                .unwrap_or(false)
                            || record
                                .application_identities
                                .iter()
                                .any(|identity| identity.value.to_ascii_lowercase().contains(needle))
                            || record
                                .hardware_identities
                                .iter()
                                .any(|identity| identity.value.to_ascii_lowercase().contains(needle))
                            || record
                                .aliases
                                .iter()
                                .any(|alias| alias.to_ascii_lowercase().contains(needle))
                            || record
                                .relations
                                .iter()
                                .any(|relation| {
                                    relation.relation.to_ascii_lowercase().contains(needle)
                                        || relation.target.to_ascii_lowercase().contains(needle)
                                })
                    })
                    .unwrap_or(true)
        })
        .collect()
}

fn validate_parent_chain(
    record: &SemanticRecord,
    existing_fqdns: &[String],
    zone_suffix: &str,
) -> Result<(), AppError> {
    let missing = record
        .hierarchy_parent_segments()
        .into_iter()
        .map(|segments| build_semantic_name(&segments, &[], zone_suffix))
        .filter(|fqdn| {
            !existing_fqdns
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(fqdn))
        })
        .collect::<Vec<_>>();

    if missing.is_empty() {
        Ok(())
    } else {
        Err(AppError::Validation(format!(
            "missing parent hierarchy records for {}: {}",
            record.fqdn,
            missing.join(", ")
        )))
    }
}

fn lease_matches_record(record: &SemanticRecord, lease: &DhcpLease) -> bool {
    lease
        .dns_name
        .as_deref()
        .map(|dns_name| record.fqdn.eq_ignore_ascii_case(dns_name))
        .unwrap_or(false)
        || record.internal_ip.as_deref() == Some(lease.address.as_str())
        || lease
            .external_ip
            .as_deref()
            .map(|external_ip| record.external_ip.as_deref() == Some(external_ip))
            .unwrap_or(false)
}

fn hydrate_record_from_lease(record: &mut SemanticRecord, lease: &DhcpLease) {
    let normalized_mac = normalize_mac(&lease.mac);
    if record.mac.is_none() && !normalized_mac.is_empty() {
        record.mac = Some(normalized_mac.clone());
        record.field_sources.insert(
            "mac".to_string(),
            MetadataField {
                value: normalized_mac.clone(),
                source: ObservationSource::DhcpFingerprint,
                updated_at: lease.last_seen,
            },
        );
    }

    if !normalized_mac.is_empty()
        && !record.hardware_identities.iter().any(|identity| {
            identity.kind == HardwareIdentityKind::MacAddress
                && identity.value.eq_ignore_ascii_case(&normalized_mac)
        })
    {
        record.hardware_identities.push(HardwareIdentity {
            kind: HardwareIdentityKind::MacAddress,
            value: normalized_mac,
            label: Some("dhcp".to_string()),
        });
        record.hardware_identities.sort_by(|left, right| left.value.cmp(&right.value));
        record.field_sources.insert(
            "hardware_identities".to_string(),
            MetadataField {
                value: record
                    .hardware_identities
                    .iter()
                    .map(|identity| format!("{}={}", identity.kind.as_str(), identity.value))
                    .collect::<Vec<_>>()
                    .join(","),
                source: ObservationSource::DhcpFingerprint,
                updated_at: lease.last_seen,
            },
        );
    }

    if record.internal_ip.is_none() {
        record.internal_ip = Some(lease.address.clone());
        record.field_sources.insert(
            "internal_ip".to_string(),
            MetadataField {
                value: lease.address.clone(),
                source: ObservationSource::DhcpFingerprint,
                updated_at: lease.last_seen,
            },
        );
    }

    if record.external_ip.is_none() {
        if let Some(external_ip) = &lease.external_ip {
            record.external_ip = Some(external_ip.clone());
            record.field_sources.insert(
                "external_ip".to_string(),
                MetadataField {
                    value: external_ip.clone(),
                    source: ObservationSource::DhcpFingerprint,
                    updated_at: lease.last_seen,
                },
            );
        }
    }

    record.updated_at = lease.last_seen;
}

fn normalize_mac(value: &str) -> String {
    let hex = value
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect::<String>()
        .to_ascii_lowercase();
    if hex.len() != 12 {
        return String::new();
    }
    hex.as_bytes()
        .chunks(2)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(":")
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sdns_common::{DeviceId, LeaseId, ObservationId};
    use sdns_core::{Isa95NodeKind, Isa95WorkCenterKind, ObservationSource};
    use sdns_dhcp::DhcpLease;

    use super::{InMemoryStore, SemanticStore};

    #[tokio::test]
    async fn in_memory_store_resolves_semantic_record() {
        let store = InMemoryStore::new();
        let record = store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::DhcpFingerprint,
                node_kind: Isa95NodeKind::Device,
                external_ip: Some("10.50.3.47".to_string()),
                internal_ip: Some("192.168.1.47".to_string()),
                class: Some("vfd".to_string()),
                vendor: Some("rockwell".to_string()),
                model: Some("PowerFlex500".to_string()),
                protocols: Some(vec!["ethernet-ip".to_string()]),
                mac: Some("00:00:BC:3A:47:12".to_string()),
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone3".to_string()),
                work_center: Some("Cell5".to_string()),
                work_center_kind: Some(Isa95WorkCenterKind::ProcessCell),
                work_unit: Some("Conveyor".to_string()),
                facility: None,
                zone: Some("Zone3".to_string()),
                cell: Some("Cell5".to_string()),
                process: Some("Conveyor".to_string()),
                function: Some("DriveVFD".to_string()),
                hardware_identities: None,
                application_identities: Some(vec![sdns_core::ApplicationIdentity {
                    kind: sdns_core::ApplicationIdentityKind::Urn,
                    value: "urn:sdns:device:drivevfd".to_string(),
                    label: Some("stable-id".to_string()),
                }]),
                aliases: Some(vec!["pf525-conveyor".to_string()]),
                relations: Some(vec![sdns_core::SemanticRelation {
                    relation: "located-in".to_string(),
                    target: "urn:isa95:work-unit:conveyor".to_string(),
                    label: None,
                }]),
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect("insert");

        let resolved = store
            .resolve("10.50.3.47")
            .await
            .expect("resolve")
            .expect("record");

        assert_eq!(record.fqdn, resolved.fqdn);
        let resolved_by_urn = store
            .resolve("urn:sdns:device:drivevfd")
            .await
            .expect("resolve by urn")
            .expect("record");
        assert_eq!(record.fqdn, resolved_by_urn.fqdn);
        let resolved_by_alias = store
            .resolve("pf525-conveyor")
            .await
            .expect("resolve by alias")
            .expect("record");
        assert_eq!(record.fqdn, resolved_by_alias.fqdn);
    }

    #[tokio::test]
    async fn in_memory_store_filters_by_isa95_hierarchy() {
        let store = InMemoryStore::new();
        store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::ManualApi,
                node_kind: Isa95NodeKind::Site,
                external_ip: Some("10.50.0.1".to_string()),
                internal_ip: Some("192.168.0.1".to_string()),
                class: Some("core-router".to_string()),
                vendor: Some("cisco".to_string()),
                model: None,
                protocols: Some(vec!["routing".to_string()]),
                mac: None,
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: None,
                work_center: None,
                work_center_kind: None,
                work_unit: None,
                facility: None,
                zone: None,
                cell: None,
                process: None,
                function: None,
                hardware_identities: Some(vec![sdns_core::HardwareIdentity {
                    kind: sdns_core::HardwareIdentityKind::SerialNumber,
                    value: "RTR-MKE-CORE-01".to_string(),
                    label: Some("serial".to_string()),
                }]),
                application_identities: Some(vec![sdns_core::ApplicationIdentity {
                    kind: sdns_core::ApplicationIdentityKind::Urn,
                    value: "urn:site:milwaukee".to_string(),
                    label: Some("site".to_string()),
                }]),
                aliases: Some(vec!["milwaukee-core".to_string()]),
                relations: None,
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect("site insert");
        store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::ManualApi,
                node_kind: Isa95NodeKind::Area,
                external_ip: Some("10.50.3.1".to_string()),
                internal_ip: Some("192.168.3.1".to_string()),
                class: Some("distribution-switch".to_string()),
                vendor: Some("cisco".to_string()),
                model: None,
                protocols: Some(vec!["switching".to_string()]),
                mac: None,
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone3".to_string()),
                work_center: None,
                work_center_kind: None,
                work_unit: None,
                facility: None,
                zone: Some("Zone3".to_string()),
                cell: None,
                process: None,
                function: None,
                hardware_identities: Some(vec![sdns_core::HardwareIdentity {
                    kind: sdns_core::HardwareIdentityKind::SerialNumber,
                    value: "SW-ZONE3-01".to_string(),
                    label: Some("serial".to_string()),
                }]),
                application_identities: None,
                aliases: Some(vec!["zone3-dist".to_string()]),
                relations: Some(vec![sdns_core::SemanticRelation {
                    relation: "uplinks-to".to_string(),
                    target: "urn:site:milwaukee".to_string(),
                    label: None,
                }]),
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect("area insert");
        store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::ManualApi,
                node_kind: Isa95NodeKind::WorkCenter,
                external_ip: Some("10.50.3.10".to_string()),
                internal_ip: Some("192.168.3.10".to_string()),
                class: Some("line-switch".to_string()),
                vendor: Some("cisco".to_string()),
                model: None,
                protocols: Some(vec!["switching".to_string()]),
                mac: None,
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone3".to_string()),
                work_center: Some("Cell5".to_string()),
                work_center_kind: Some(Isa95WorkCenterKind::ProcessCell),
                work_unit: None,
                facility: None,
                zone: None,
                cell: Some("Cell5".to_string()),
                process: None,
                function: None,
                hardware_identities: Some(vec![sdns_core::HardwareIdentity {
                    kind: sdns_core::HardwareIdentityKind::SerialNumber,
                    value: "SW-CELL5-01".to_string(),
                    label: Some("serial".to_string()),
                }]),
                application_identities: None,
                aliases: Some(vec!["cell5-line-switch".to_string()]),
                relations: None,
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect("work center insert");
        store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::ManualApi,
                node_kind: Isa95NodeKind::WorkUnit,
                external_ip: Some("10.50.3.20".to_string()),
                internal_ip: Some("192.168.3.20".to_string()),
                class: Some("machine-switch".to_string()),
                vendor: Some("cisco".to_string()),
                model: None,
                protocols: Some(vec!["switching".to_string()]),
                mac: None,
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone3".to_string()),
                work_center: Some("Cell5".to_string()),
                work_center_kind: Some(Isa95WorkCenterKind::ProcessCell),
                work_unit: Some("Conveyor".to_string()),
                facility: None,
                zone: None,
                cell: Some("Cell5".to_string()),
                process: Some("Conveyor".to_string()),
                function: None,
                hardware_identities: Some(vec![sdns_core::HardwareIdentity {
                    kind: sdns_core::HardwareIdentityKind::X509SanUri,
                    value: "spiffe://plant/milwaukee/packout/conveyor-switch".to_string(),
                    label: Some("future-cert".to_string()),
                }]),
                application_identities: Some(vec![sdns_core::ApplicationIdentity {
                    kind: sdns_core::ApplicationIdentityKind::Uni,
                    value: "uni://switches/cell5/conveyor".to_string(),
                    label: Some("switch-profile".to_string()),
                }]),
                aliases: Some(vec!["conveyor-machine-switch".to_string()]),
                relations: None,
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect("work unit insert");
        store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::ManualApi,
                node_kind: Isa95NodeKind::Device,
                external_ip: Some("10.50.3.47".to_string()),
                internal_ip: Some("192.168.1.47".to_string()),
                class: Some("vfd".to_string()),
                vendor: Some("rockwell".to_string()),
                model: Some("PowerFlex500".to_string()),
                protocols: Some(vec!["ethernet-ip".to_string()]),
                mac: Some("00:00:BC:3A:47:12".to_string()),
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone3".to_string()),
                work_center: Some("Cell5".to_string()),
                work_center_kind: Some(Isa95WorkCenterKind::ProcessCell),
                work_unit: Some("Conveyor".to_string()),
                facility: None,
                zone: None,
                cell: None,
                process: None,
                function: Some("DriveVFD".to_string()),
                hardware_identities: None,
                application_identities: Some(vec![sdns_core::ApplicationIdentity {
                    kind: sdns_core::ApplicationIdentityKind::Urn,
                    value: "urn:sdns:device:drivevfd".to_string(),
                    label: Some("stable-id".to_string()),
                }]),
                aliases: Some(vec!["pf525-conveyor".to_string()]),
                relations: Some(vec![sdns_core::SemanticRelation {
                    relation: "served-by".to_string(),
                    target: "uni://switches/cell5/conveyor".to_string(),
                    label: None,
                }]),
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect("insert");

        let matches = store
            .query(sdns_core::RecordFilter {
                node_kind: Some(Isa95NodeKind::Device),
                site: Some("Milwaukee".to_string()),
                work_center: Some("Cell5".to_string()),
                work_unit: Some("Conveyor".to_string()),
                ..Default::default()
            })
            .await
            .expect("query");

        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].work_center_kind,
            Some(Isa95WorkCenterKind::ProcessCell)
        );
    }

    #[tokio::test]
    async fn manual_device_insert_requires_parent_hierarchy_records() {
        let store = InMemoryStore::new();

        let error = store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::ManualApi,
                node_kind: Isa95NodeKind::Device,
                external_ip: Some("10.50.4.99".to_string()),
                internal_ip: Some("192.168.2.99".to_string()),
                class: Some("robot".to_string()),
                vendor: Some("fanuc".to_string()),
                model: Some("M-20iD".to_string()),
                protocols: Some(vec!["ethernet-ip".to_string()]),
                mac: Some("ac:de:48:00:99:11".to_string()),
                switch_port: Some("Gi1/0/24".to_string()),
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone4".to_string()),
                work_center: Some("Packout".to_string()),
                work_center_kind: Some(Isa95WorkCenterKind::ProductionLine),
                work_unit: Some("Palletizer".to_string()),
                facility: None,
                zone: Some("Zone4".to_string()),
                cell: None,
                process: Some("Palletizer".to_string()),
                function: Some("CaseRobot".to_string()),
                hardware_identities: None,
                application_identities: Some(vec![sdns_core::ApplicationIdentity {
                    kind: sdns_core::ApplicationIdentityKind::Urn,
                    value: "urn:mes:asset:case-robot".to_string(),
                    label: Some("mes".to_string()),
                }]),
                aliases: Some(vec!["case-robot".to_string()]),
                relations: None,
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect_err("missing parents should be rejected");

        assert!(error
            .to_string()
            .contains("missing parent hierarchy records"));
    }

    #[tokio::test]
    async fn lease_enriches_record_with_mac_hardware_identity() {
        let store = InMemoryStore::new();
        let record = store
            .upsert_observation(sdns_core::Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::Discovery,
                node_kind: Isa95NodeKind::Device,
                external_ip: Some("10.50.3.47".to_string()),
                internal_ip: Some("192.168.1.47".to_string()),
                class: Some("vfd".to_string()),
                vendor: Some("rockwell".to_string()),
                model: Some("PowerFlex500".to_string()),
                protocols: Some(vec!["ethernet-ip".to_string()]),
                mac: None,
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone3".to_string()),
                work_center: Some("Cell5".to_string()),
                work_center_kind: Some(Isa95WorkCenterKind::ProcessCell),
                work_unit: Some("Conveyor".to_string()),
                facility: None,
                zone: Some("Zone3".to_string()),
                cell: Some("Cell5".to_string()),
                process: Some("Conveyor".to_string()),
                function: Some("DriveVFD".to_string()),
                hardware_identities: None,
                application_identities: None,
                aliases: None,
                relations: None,
                status: Some(sdns_core::RecordStatus::Active),
            })
            .await
            .expect("insert");

        store
            .upsert_lease(DhcpLease {
                id: LeaseId::new(),
                mac: "00-00-BC-3A-47-12".to_string(),
                address: "192.168.1.47".to_string(),
                external_ip: Some("10.50.3.47".to_string()),
                dns_name: Some(record.fqdn.clone()),
                role: Some("conveyor-vfd".to_string()),
                class: Some("vfd".to_string()),
                switch_port: Some("Gi1/0/5".to_string()),
                status: sdns_core::RecordStatus::Active,
                first_seen: Utc::now(),
                last_seen: Utc::now(),
            })
            .await
            .expect("lease insert");

        let enriched = store
            .resolve(&record.fqdn)
            .await
            .expect("resolve")
            .expect("record");

        assert_eq!(enriched.mac.as_deref(), Some("00:00:bc:3a:47:12"));
        assert!(enriched
            .hardware_identities
            .iter()
            .any(|identity| identity.value == "00:00:bc:3a:47:12"));
    }
}
