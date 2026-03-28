use chrono::{DateTime, Utc};
use sdns_common::{LeaseId, QuarantineEntryId};
use sdns_core::RecordStatus;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DhcpLease {
    pub id: LeaseId,
    pub mac: String,
    pub address: String,
    pub external_ip: Option<String>,
    pub dns_name: Option<String>,
    pub role: Option<String>,
    pub class: Option<String>,
    pub switch_port: Option<String>,
    pub status: RecordStatus,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuarantineEntry {
    pub id: QuarantineEntryId,
    pub mac: String,
    pub fingerprint_summary: Option<String>,
    pub switch_port: Option<String>,
    pub reason: String,
    pub queued_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LeaseDecision {
    RoleBased {
        address: String,
        role: String,
        process_area: String,
    },
    Unassigned {
        address: Option<String>,
    },
    Quarantine {
        address: Option<String>,
        reason: String,
    },
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthorizeQuarantineRequest {
    pub mac: String,
    pub assigned_role: String,
    pub justification: String,
    pub operator: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplacementOutcome {
    pub previous_mac: String,
    pub new_mac: String,
    pub preserved_role: String,
    pub preserved_address: String,
}

pub fn detect_replacement(
    previous: &DhcpLease,
    new_mac: &str,
    new_class: Option<&str>,
    switch_port: Option<&str>,
) -> Option<ReplacementOutcome> {
    if previous.mac.eq_ignore_ascii_case(new_mac) {
        return None;
    }

    let same_port = previous.switch_port.as_deref() == switch_port;
    let same_class = previous.class.as_deref() == new_class;
    let has_role = previous.role.is_some();

    if same_port && same_class && has_role {
        Some(ReplacementOutcome {
            previous_mac: previous.mac.clone(),
            new_mac: new_mac.to_string(),
            preserved_role: previous.role.clone().unwrap_or_default(),
            preserved_address: previous.address.clone(),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sdns_common::LeaseId;

    use super::{DhcpLease, detect_replacement};
    use sdns_core::RecordStatus;

    #[test]
    fn detects_like_for_like_replacement() {
        let previous = DhcpLease {
            id: LeaseId::new(),
            mac: "00:00:BC:2F:33:A8".to_string(),
            address: "192.168.1.47".to_string(),
            external_ip: None,
            dns_name: Some("DriveVFD.Conveyor.Cell5.local".to_string()),
            role: Some("conveyor-vfd".to_string()),
            class: Some("vfd".to_string()),
            switch_port: Some("Gi1/0/5".to_string()),
            status: RecordStatus::Active,
            first_seen: Utc::now(),
            last_seen: Utc::now(),
        };

        let replacement =
            detect_replacement(&previous, "00:00:BC:3A:47:12", Some("vfd"), Some("Gi1/0/5"))
                .expect("replacement");

        assert_eq!(replacement.preserved_address, "192.168.1.47");
    }
}
