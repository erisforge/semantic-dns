use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sdns_common::{DeviceId, ObservationId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObservationSource {
    ManualApi,
    ProtocolAnalysis,
    SwitchIntelligence,
    DhcpFingerprint,
    Discovery,
    ReplacementInference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidenceLevel {
    Authoritative,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RecordStatus {
    Active,
    Released,
    Expired,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Isa95WorkCenterKind {
    ProcessCell,
    Unit,
    ProductionLine,
    WorkCell,
    ProductionUnit,
    StorageZone,
    StorageUnit,
    WorkCenter,
}

impl Isa95WorkCenterKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProcessCell => "process-cell",
            Self::Unit => "unit",
            Self::ProductionLine => "production-line",
            Self::WorkCell => "work-cell",
            Self::ProductionUnit => "production-unit",
            Self::StorageZone => "storage-zone",
            Self::StorageUnit => "storage-unit",
            Self::WorkCenter => "work-center",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Isa95NodeKind {
    Site,
    Area,
    WorkCenter,
    WorkUnit,
    Device,
}

impl Default for Isa95NodeKind {
    fn default() -> Self {
        Self::Device
    }
}

impl Isa95NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::Area => "area",
            Self::WorkCenter => "work-center",
            Self::WorkUnit => "work-unit",
            Self::Device => "device",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplicationIdentityKind {
    Uni,
    Urn,
}

impl ApplicationIdentityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Uni => "uni",
            Self::Urn => "urn",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ApplicationIdentity {
    pub kind: ApplicationIdentityKind,
    pub value: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HardwareIdentityKind {
    MacAddress,
    SerialNumber,
    DhcpClientId,
    X509Subject,
    X509SanUri,
    X509SpkiSha256,
}

impl HardwareIdentityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MacAddress => "mac-address",
            Self::SerialNumber => "serial-number",
            Self::DhcpClientId => "dhcp-client-id",
            Self::X509Subject => "x509-subject",
            Self::X509SanUri => "x509-san-uri",
            Self::X509SpkiSha256 => "x509-spki-sha256",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HardwareIdentity {
    pub kind: HardwareIdentityKind,
    pub value: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SemanticRelation {
    pub relation: String,
    pub target: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetadataField {
    pub value: String,
    pub source: ObservationSource,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub id: ObservationId,
    pub device_id: DeviceId,
    pub observed_at: DateTime<Utc>,
    pub source: ObservationSource,
    #[serde(default)]
    pub node_kind: Isa95NodeKind,
    pub external_ip: Option<String>,
    pub internal_ip: Option<String>,
    pub class: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub protocols: Option<Vec<String>>,
    pub mac: Option<String>,
    pub switch_port: Option<String>,
    pub enterprise: Option<String>,
    pub site: Option<String>,
    pub area: Option<String>,
    pub work_center: Option<String>,
    pub work_center_kind: Option<Isa95WorkCenterKind>,
    pub work_unit: Option<String>,
    pub facility: Option<String>,
    pub zone: Option<String>,
    pub cell: Option<String>,
    pub process: Option<String>,
    pub function: Option<String>,
    pub hardware_identities: Option<Vec<HardwareIdentity>>,
    pub application_identities: Option<Vec<ApplicationIdentity>>,
    pub aliases: Option<Vec<String>>,
    pub relations: Option<Vec<SemanticRelation>>,
    pub status: Option<RecordStatus>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticRecord {
    pub device_id: DeviceId,
    pub fqdn: String,
    #[serde(default)]
    pub node_kind: Isa95NodeKind,
    pub external_ip: Option<String>,
    pub internal_ip: Option<String>,
    pub class: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub protocols: Vec<String>,
    pub mac: Option<String>,
    pub switch_port: Option<String>,
    pub enterprise: Option<String>,
    pub site: Option<String>,
    pub area: Option<String>,
    pub work_center: Option<String>,
    pub work_center_kind: Option<Isa95WorkCenterKind>,
    pub work_unit: Option<String>,
    pub facility: Option<String>,
    pub zone: Option<String>,
    pub cell: Option<String>,
    pub process: Option<String>,
    pub function: Option<String>,
    #[serde(default)]
    pub hardware_identities: Vec<HardwareIdentity>,
    #[serde(default)]
    pub application_identities: Vec<ApplicationIdentity>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub relations: Vec<SemanticRelation>,
    pub status: RecordStatus,
    pub updated_at: DateTime<Utc>,
    pub field_sources: BTreeMap<String, MetadataField>,
}

impl SemanticRecord {
    pub fn new(device_id: DeviceId, fqdn: String, updated_at: DateTime<Utc>) -> Self {
        Self {
            device_id,
            fqdn,
            node_kind: Isa95NodeKind::Device,
            external_ip: None,
            internal_ip: None,
            class: None,
            vendor: None,
            model: None,
            protocols: Vec::new(),
            mac: None,
            switch_port: None,
            enterprise: None,
            site: None,
            area: None,
            work_center: None,
            work_center_kind: None,
            work_unit: None,
            facility: None,
            zone: None,
            cell: None,
            process: None,
            function: None,
            hardware_identities: Vec::new(),
            application_identities: Vec::new(),
            aliases: Vec::new(),
            relations: Vec::new(),
            status: RecordStatus::Active,
            updated_at,
            field_sources: BTreeMap::new(),
        }
    }

    pub fn confidence(&self) -> ConfidenceLevel {
        self.field_sources
            .values()
            .map(|entry| match entry.source {
                ObservationSource::ManualApi => ConfidenceLevel::Authoritative,
                ObservationSource::ProtocolAnalysis => ConfidenceLevel::High,
                ObservationSource::SwitchIntelligence => ConfidenceLevel::High,
                ObservationSource::DhcpFingerprint => ConfidenceLevel::Medium,
                ObservationSource::ReplacementInference => ConfidenceLevel::Medium,
                ObservationSource::Discovery => ConfidenceLevel::Low,
            })
            .max_by_key(|level| match level {
                ConfidenceLevel::Low => 0,
                ConfidenceLevel::Medium => 1,
                ConfidenceLevel::High => 2,
                ConfidenceLevel::Authoritative => 3,
            })
            .unwrap_or(ConfidenceLevel::Low)
    }

    pub fn effective_site(&self) -> Option<&str> {
        self.site.as_deref().or(self.facility.as_deref())
    }

    pub fn effective_area(&self) -> Option<&str> {
        self.area.as_deref().or(self.zone.as_deref())
    }

    pub fn effective_work_center(&self) -> Option<&str> {
        self.work_center.as_deref().or(self.cell.as_deref())
    }

    pub fn effective_work_unit(&self) -> Option<&str> {
        self.work_unit.as_deref().or(self.process.as_deref())
    }

    pub fn effective_leaf(&self) -> Option<&str> {
        self.function.as_deref().or(self.class.as_deref())
    }

    pub fn label_for_kind(&self) -> Option<&str> {
        match self.node_kind {
            Isa95NodeKind::Site => self.effective_site(),
            Isa95NodeKind::Area => self.effective_area(),
            Isa95NodeKind::WorkCenter => self.effective_work_center(),
            Isa95NodeKind::WorkUnit => self.effective_work_unit(),
            Isa95NodeKind::Device => self.effective_leaf(),
        }
    }

    pub fn naming_segments(&self) -> Vec<&str> {
        let mut segments = Vec::with_capacity(5);
        match self.node_kind {
            Isa95NodeKind::Site => {
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::Area => {
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::WorkCenter => {
                if let Some(work_center) = self.effective_work_center() {
                    segments.push(work_center);
                }
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::WorkUnit => {
                if let Some(work_unit) = self.effective_work_unit() {
                    segments.push(work_unit);
                }
                if let Some(work_center) = self.effective_work_center() {
                    segments.push(work_center);
                }
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::Device => {
                if let Some(leaf) = self.effective_leaf() {
                    segments.push(leaf);
                }
                if let Some(work_unit) = self.effective_work_unit() {
                    segments.push(work_unit);
                }
                if let Some(work_center) = self.effective_work_center() {
                    segments.push(work_center);
                }
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
        }
        segments
    }

    pub fn hierarchy_parent_segments(&self) -> Vec<Vec<&str>> {
        match self.node_kind {
            Isa95NodeKind::Site => Vec::new(),
            Isa95NodeKind::Area => self
                .effective_site()
                .map(|site| vec![vec![site]])
                .unwrap_or_default(),
            Isa95NodeKind::WorkCenter => {
                let mut parents = Vec::new();
                if let (Some(area), Some(site)) = (self.effective_area(), self.effective_site()) {
                    parents.push(vec![area, site]);
                    parents.push(vec![site]);
                }
                parents
            }
            Isa95NodeKind::WorkUnit => {
                let mut parents = Vec::new();
                if let (Some(work_center), Some(area), Some(site)) = (
                    self.effective_work_center(),
                    self.effective_area(),
                    self.effective_site(),
                ) {
                    parents.push(vec![work_center, area, site]);
                    parents.push(vec![area, site]);
                    parents.push(vec![site]);
                }
                parents
            }
            Isa95NodeKind::Device => {
                let mut parents = Vec::new();
                if let (Some(work_unit), Some(work_center), Some(area), Some(site)) = (
                    self.effective_work_unit(),
                    self.effective_work_center(),
                    self.effective_area(),
                    self.effective_site(),
                ) {
                    parents.push(vec![work_unit, work_center, area, site]);
                    parents.push(vec![work_center, area, site]);
                    parents.push(vec![area, site]);
                    parents.push(vec![site]);
                }
                parents
            }
        }
    }

    pub fn has_application_identity(&self, value: &str) -> bool {
        self.application_identities
            .iter()
            .any(|identity| identity.value.eq_ignore_ascii_case(value))
    }

    pub fn has_alias(&self, value: &str) -> bool {
        self.aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(value))
    }

    pub fn has_hardware_identity(&self, value: &str) -> bool {
        self.hardware_identities
            .iter()
            .any(|identity| identity.value.eq_ignore_ascii_case(value))
    }
}

impl Observation {
    pub fn effective_site(&self) -> Option<&str> {
        self.site.as_deref().or(self.facility.as_deref())
    }

    pub fn effective_area(&self) -> Option<&str> {
        self.area.as_deref().or(self.zone.as_deref())
    }

    pub fn effective_work_center(&self) -> Option<&str> {
        self.work_center.as_deref().or(self.cell.as_deref())
    }

    pub fn effective_work_unit(&self) -> Option<&str> {
        self.work_unit.as_deref().or(self.process.as_deref())
    }

    pub fn effective_leaf(&self) -> Option<&str> {
        self.function.as_deref().or(self.class.as_deref())
    }

    pub fn label_for_kind(&self) -> Option<&str> {
        match self.node_kind {
            Isa95NodeKind::Site => self.effective_site(),
            Isa95NodeKind::Area => self.effective_area(),
            Isa95NodeKind::WorkCenter => self.effective_work_center(),
            Isa95NodeKind::WorkUnit => self.effective_work_unit(),
            Isa95NodeKind::Device => self.effective_leaf(),
        }
    }

    pub fn naming_segments(&self) -> Vec<&str> {
        let mut segments = Vec::with_capacity(5);
        match self.node_kind {
            Isa95NodeKind::Site => {
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::Area => {
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::WorkCenter => {
                if let Some(work_center) = self.effective_work_center() {
                    segments.push(work_center);
                }
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::WorkUnit => {
                if let Some(work_unit) = self.effective_work_unit() {
                    segments.push(work_unit);
                }
                if let Some(work_center) = self.effective_work_center() {
                    segments.push(work_center);
                }
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
            Isa95NodeKind::Device => {
                if let Some(leaf) = self.effective_leaf() {
                    segments.push(leaf);
                }
                if let Some(work_unit) = self.effective_work_unit() {
                    segments.push(work_unit);
                }
                if let Some(work_center) = self.effective_work_center() {
                    segments.push(work_center);
                }
                if let Some(area) = self.effective_area() {
                    segments.push(area);
                }
                if let Some(site) = self.effective_site() {
                    segments.push(site);
                }
            }
        }
        segments
    }

    pub fn validate_isa95_path(&self) -> Result<(), String> {
        let missing = match self.node_kind {
            Isa95NodeKind::Site => missing_labels(&[("site", self.effective_site())]),
            Isa95NodeKind::Area => missing_labels(&[
                ("area", self.effective_area()),
                ("site", self.effective_site()),
            ]),
            Isa95NodeKind::WorkCenter => missing_labels(&[
                ("work_center", self.effective_work_center()),
                ("area", self.effective_area()),
                ("site", self.effective_site()),
            ]),
            Isa95NodeKind::WorkUnit => missing_labels(&[
                ("work_unit", self.effective_work_unit()),
                ("work_center", self.effective_work_center()),
                ("area", self.effective_area()),
                ("site", self.effective_site()),
            ]),
            Isa95NodeKind::Device => missing_labels(&[
                ("leaf", self.effective_leaf()),
                ("work_unit", self.effective_work_unit()),
                ("work_center", self.effective_work_center()),
                ("area", self.effective_area()),
                ("site", self.effective_site()),
            ]),
        };

        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} records require {}",
                self.node_kind.as_str(),
                missing.join(", ")
            ))
        }
    }

    pub fn validate_application_identity_payload(&self) -> Result<(), String> {
        if let Some(identities) = &self.hardware_identities {
            for identity in identities {
                validate_hardware_identity(identity)?;
            }
        }
        if let Some(identities) = &self.application_identities {
            for identity in identities {
                validate_application_identity(identity)?;
            }
        }
        if let Some(aliases) = &self.aliases {
            for alias in aliases {
                if normalize_alias(alias).is_empty() {
                    return Err("aliases must not be empty".to_string());
                }
            }
        }
        if let Some(relations) = &self.relations {
            for relation in relations {
                if normalize_relation_value(&relation.relation).is_empty() {
                    return Err("relations must include a non-empty relation name".to_string());
                }
                if relation.target.trim().is_empty() {
                    return Err("relations must include a non-empty target".to_string());
                }
            }
        }
        Ok(())
    }

    pub fn validate_hardware_requirement(&self) -> Result<(), String> {
        let has_mac = self
            .mac
            .as_ref()
            .map(|value| !normalize_mac_like(value).is_empty())
            .unwrap_or(false);
        let has_hardware_identity = self
            .hardware_identities
            .as_ref()
            .map(|identities| !identities.is_empty())
            .unwrap_or(false);

        if has_mac || has_hardware_identity {
            Ok(())
        } else {
            Err(format!(
                "{} records require a MAC address or another hardware identity",
                self.node_kind.as_str()
            ))
        }
    }
}

fn missing_labels(required: &[(&str, Option<&str>)]) -> Vec<String> {
    required
        .iter()
        .filter_map(|(label, value)| value.is_none().then_some((*label).to_string()))
        .collect()
}

pub fn validate_application_identity(identity: &ApplicationIdentity) -> Result<(), String> {
    let value = identity.value.trim();
    if value.is_empty() {
        return Err("application identity values must not be empty".to_string());
    }
    match identity.kind {
        ApplicationIdentityKind::Urn => {
            if !value.to_ascii_lowercase().starts_with("urn:") {
                return Err(format!("URN identities must start with `urn:`: {value}"));
            }
        }
        ApplicationIdentityKind::Uni => {
            if value.chars().any(char::is_whitespace) {
                return Err(format!("UNI identities must not contain whitespace: {value}"));
            }
        }
    }
    Ok(())
}

pub fn validate_hardware_identity(identity: &HardwareIdentity) -> Result<(), String> {
    let value = identity.value.trim();
    if value.is_empty() {
        return Err("hardware identity values must not be empty".to_string());
    }
    match identity.kind {
        HardwareIdentityKind::MacAddress => {
            let normalized = normalize_mac_like(value);
            if normalized.len() != 17 {
                return Err(format!("MAC identities must be valid MAC addresses: {value}"));
            }
        }
        HardwareIdentityKind::SerialNumber
        | HardwareIdentityKind::DhcpClientId
        | HardwareIdentityKind::X509Subject
        | HardwareIdentityKind::X509SanUri
        | HardwareIdentityKind::X509SpkiSha256 => {}
    }
    Ok(())
}

pub fn normalize_application_identity(identity: &ApplicationIdentity) -> ApplicationIdentity {
    ApplicationIdentity {
        kind: identity.kind,
        value: match identity.kind {
            ApplicationIdentityKind::Urn => identity.value.trim().to_ascii_lowercase(),
            ApplicationIdentityKind::Uni => identity.value.trim().to_string(),
        },
        label: identity.label.as_ref().map(|label| label.trim().to_string()).filter(|label| !label.is_empty()),
    }
}

pub fn normalize_hardware_identity(identity: &HardwareIdentity) -> HardwareIdentity {
    HardwareIdentity {
        kind: identity.kind,
        value: match identity.kind {
            HardwareIdentityKind::MacAddress => normalize_mac_like(&identity.value),
            HardwareIdentityKind::X509SpkiSha256 => identity.value.trim().to_ascii_lowercase(),
            HardwareIdentityKind::SerialNumber
            | HardwareIdentityKind::DhcpClientId
            | HardwareIdentityKind::X509Subject
            | HardwareIdentityKind::X509SanUri => identity.value.trim().to_string(),
        },
        label: identity.label.as_ref().map(|label| label.trim().to_string()).filter(|label| !label.is_empty()),
    }
}

pub fn normalize_alias(value: &str) -> String {
    value.trim().to_string()
}

pub fn normalize_relation(relation: &SemanticRelation) -> SemanticRelation {
    SemanticRelation {
        relation: normalize_relation_value(&relation.relation),
        target: relation.target.trim().to_string(),
        label: relation.label.as_ref().map(|label| label.trim().to_string()).filter(|label| !label.is_empty()),
    }
}

fn normalize_relation_value(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_mac_like(value: &str) -> String {
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

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RecordFilter {
    pub q: Option<String>,
    pub node_kind: Option<Isa95NodeKind>,
    pub hardware_identity: Option<String>,
    pub application_id: Option<String>,
    pub alias: Option<String>,
    pub class: Option<String>,
    pub vendor: Option<String>,
    pub enterprise: Option<String>,
    pub site: Option<String>,
    pub area: Option<String>,
    pub work_center: Option<String>,
    pub work_center_kind: Option<Isa95WorkCenterKind>,
    pub work_unit: Option<String>,
    pub cell: Option<String>,
    pub zone: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncStatus {
    pub total_leases: usize,
    pub dns_records_synced: usize,
    pub pending_updates: usize,
    pub failed_updates: usize,
    pub last_reconciliation: Option<DateTime<Utc>>,
}
