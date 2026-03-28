use async_trait::async_trait;
use sdns_common::AppError;
use sdns_core::{RecordFilter, SemanticRecord};
use sdns_store::SemanticStore;
use std::sync::Arc;

#[async_trait]
pub trait DnsPublisher: Send + Sync {
    async fn publish(&self, record: &SemanticRecord) -> Result<(), AppError>;
}

#[derive(Clone)]
pub struct FileDnsPublisher {
    zone_file: String,
    zone_name: String,
    store: Arc<dyn SemanticStore>,
}

impl FileDnsPublisher {
    pub fn new(
        zone_name: impl Into<String>,
        zone_file: impl Into<String>,
        store: Arc<dyn SemanticStore>,
    ) -> Self {
        Self {
            zone_name: zone_name.into(),
            zone_file: zone_file.into(),
            store,
        }
    }

    pub async fn sync_all(&self) -> Result<(), AppError> {
        let records = self.store.query(RecordFilter::default()).await?;
        let rendered = render_zone_file(&self.zone_name, &records);
        tokio::fs::write(&self.zone_file, rendered)
            .await
            .map_err(|err| AppError::Internal(format!("failed to write zone file: {err}")))
    }
}

#[async_trait]
impl DnsPublisher for FileDnsPublisher {
    async fn publish(&self, _record: &SemanticRecord) -> Result<(), AppError> {
        self.sync_all().await
    }
}

fn render_zone_file(zone_name: &str, records: &[SemanticRecord]) -> String {
    let origin = fqdn_with_dot(zone_name);
    let serial = chrono::Utc::now().format("%Y%m%d%H").to_string();
    let mut lines = vec![
        format!("$ORIGIN {origin}"),
        "$TTL 300".to_string(),
        format!("@ IN SOA ns1.{origin} hostmaster.{origin} ({serial} 60 30 604800 300)"),
        format!("@ IN NS ns1.{origin}"),
    ];

    for record in records {
        let name = fqdn_with_dot(&record.fqdn);
        if let Some(external_ip) = &record.external_ip {
            lines.push(format!("{name} 300 IN A {external_ip}"));
        }
        for (key, value) in &record.field_sources {
            lines.push(format!(
                "{name} 300 IN TXT \"rta-{}={}\"",
                key.replace('_', "-"),
                escape_txt_value(&value.value)
            ));
        }
        for identity in &record.hardware_identities {
            lines.push(format!(
                "{name} 300 IN TXT \"rta-hw-id={}\"",
                escape_txt_value(&format!("{}={}", identity.kind.as_str(), identity.value))
            ));
        }
        for identity in &record.application_identities {
            lines.push(format!(
                "{name} 300 IN TXT \"rta-{}={}\"",
                identity.kind.as_str(),
                escape_txt_value(&identity.value)
            ));
        }
        for alias in &record.aliases {
            lines.push(format!(
                "{name} 300 IN TXT \"rta-alias={}\"",
                escape_txt_value(alias)
            ));
        }
        for relation in &record.relations {
            lines.push(format!(
                "{name} 300 IN TXT \"rta-relation={}\"",
                escape_txt_value(&format!("{}->{}", relation.relation, relation.target))
            ));
        }
    }

    format!("{}\n", lines.join("\n"))
}

fn fqdn_with_dot(name: &str) -> String {
    if name.ends_with('.') {
        name.to_string()
    } else {
        format!("{name}.")
    }
}

fn escape_txt_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use sdns_common::DeviceId;
    use sdns_core::{
        ApplicationIdentity, ApplicationIdentityKind, HardwareIdentity, HardwareIdentityKind,
        SemanticRecord, SemanticRelation,
    };
    use std::collections::BTreeMap;

    use super::render_zone_file;
    use chrono::Utc;

    #[test]
    fn zone_file_contains_soa_and_record_data() {
        let zone = render_zone_file(
            "local",
            &[SemanticRecord {
                device_id: DeviceId::new(),
                fqdn: "DriveVFD.Conveyor.Cell5.Zone3.Milwaukee.local".to_string(),
                node_kind: sdns_core::Isa95NodeKind::Device,
                external_ip: Some("10.50.3.47".to_string()),
                internal_ip: Some("192.168.1.47".to_string()),
                class: Some("vfd".to_string()),
                vendor: Some("rockwell".to_string()),
                model: Some("PowerFlex500".to_string()),
                protocols: vec!["ethernet-ip".to_string()],
                mac: Some("00:00:BC:3A:47:12".to_string()),
                switch_port: None,
                enterprise: Some("Butterbones".to_string()),
                site: Some("Milwaukee".to_string()),
                area: Some("Zone3".to_string()),
                work_center: Some("Cell5".to_string()),
                work_center_kind: Some(sdns_core::Isa95WorkCenterKind::ProcessCell),
                work_unit: Some("Conveyor".to_string()),
                facility: None,
                zone: Some("Zone3".to_string()),
                cell: Some("Cell5".to_string()),
                process: Some("Conveyor".to_string()),
                function: Some("DriveVFD".to_string()),
                hardware_identities: vec![HardwareIdentity {
                    kind: HardwareIdentityKind::MacAddress,
                    value: "00:00:bc:3a:47:12".to_string(),
                    label: Some("dhcp".to_string()),
                }],
                application_identities: vec![ApplicationIdentity {
                    kind: ApplicationIdentityKind::Urn,
                    value: "urn:sdns:device:drivevfd".to_string(),
                    label: Some("stable-id".to_string()),
                }],
                aliases: vec!["pf525-conveyor".to_string()],
                relations: vec![SemanticRelation {
                    relation: "located-in".to_string(),
                    target: "urn:isa95:work-unit:conveyor".to_string(),
                    label: None,
                }],
                status: sdns_core::RecordStatus::Active,
                updated_at: Utc::now(),
                field_sources: BTreeMap::from([(
                    "class".to_string(),
                    sdns_core::MetadataField {
                        value: "vfd".to_string(),
                        source: sdns_core::ObservationSource::DhcpFingerprint,
                        updated_at: Utc::now(),
                    },
                )]),
            }],
        );

        assert!(zone.contains("SOA ns1.local."));
        assert!(
            zone.contains("DriveVFD.Conveyor.Cell5.Zone3.Milwaukee.local. 300 IN A 10.50.3.47")
        );
        assert!(zone.contains("TXT \"rta-class=vfd\""));
        assert!(zone.contains("TXT \"rta-hw-id=mac-address=00:00:bc:3a:47:12\""));
        assert!(zone.contains("TXT \"rta-urn=urn:sdns:device:drivevfd\""));
        assert!(zone.contains("TXT \"rta-alias=pf525-conveyor\""));
    }
}
