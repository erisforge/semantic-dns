use chrono::Utc;
use sdns_common::AppError;

use crate::{
    ApplicationIdentity, HardwareIdentity, HardwareIdentityKind, Isa95NodeKind,
    Isa95WorkCenterKind, MetadataField, Observation, ObservationSource, SemanticRecord,
    SemanticRelation,
    naming::build_semantic_name,
};
use crate::domain::{
    normalize_alias, normalize_application_identity, normalize_hardware_identity,
    normalize_relation,
};

pub fn merge_observation(
    existing: Option<SemanticRecord>,
    observation: &Observation,
    sibling_names: &[String],
    zone_suffix: &str,
) -> Result<SemanticRecord, AppError> {
    observation
        .validate_application_identity_payload()
        .map_err(AppError::Validation)?;
    if matches!(observation.source, ObservationSource::ManualApi) {
        observation
            .validate_isa95_path()
            .map_err(AppError::Validation)?;
        observation
            .validate_hardware_requirement()
            .map_err(AppError::Validation)?;
    }
    let fallback_name =
        build_semantic_name(&observation.naming_segments(), sibling_names, zone_suffix);

    let mut record = existing.unwrap_or_else(|| {
        SemanticRecord::new(
            observation.device_id,
            fallback_name,
            observation.observed_at,
        )
    });

    merge_node_kind(&mut record.node_kind, &mut record.field_sources, observation.node_kind, observation);

    merge_field(
        &mut record.external_ip,
        &mut record.field_sources,
        "external_ip",
        observation.external_ip.as_ref(),
        observation,
    );
    merge_field(
        &mut record.internal_ip,
        &mut record.field_sources,
        "internal_ip",
        observation.internal_ip.as_ref(),
        observation,
    );
    merge_field(
        &mut record.class,
        &mut record.field_sources,
        "class",
        observation.class.as_ref(),
        observation,
    );
    merge_field(
        &mut record.vendor,
        &mut record.field_sources,
        "vendor",
        observation.vendor.as_ref(),
        observation,
    );
    merge_field(
        &mut record.model,
        &mut record.field_sources,
        "model",
        observation.model.as_ref(),
        observation,
    );
    merge_field(
        &mut record.mac,
        &mut record.field_sources,
        "mac",
        observation.mac.as_ref(),
        observation,
    );
    merge_field(
        &mut record.switch_port,
        &mut record.field_sources,
        "switch_port",
        observation.switch_port.as_ref(),
        observation,
    );
    merge_field(
        &mut record.enterprise,
        &mut record.field_sources,
        "enterprise",
        observation.enterprise.as_ref(),
        observation,
    );

    let site_value = observation.site.as_ref().or(observation.facility.as_ref());
    merge_field(
        &mut record.site,
        &mut record.field_sources,
        "site",
        site_value,
        observation,
    );
    merge_field(
        &mut record.facility,
        &mut record.field_sources,
        "facility",
        site_value,
        observation,
    );

    let area_value = observation.area.as_ref().or(observation.zone.as_ref());
    merge_field(
        &mut record.area,
        &mut record.field_sources,
        "area",
        area_value,
        observation,
    );
    merge_field(
        &mut record.zone,
        &mut record.field_sources,
        "zone",
        area_value,
        observation,
    );

    let work_center_value = observation
        .work_center
        .as_ref()
        .or(observation.cell.as_ref());
    merge_field(
        &mut record.work_center,
        &mut record.field_sources,
        "work_center",
        work_center_value,
        observation,
    );
    merge_field(
        &mut record.cell,
        &mut record.field_sources,
        "cell",
        work_center_value,
        observation,
    );

    if let Some(work_center_kind) = observation.work_center_kind {
        merge_work_center_kind(
            &mut record.work_center_kind,
            &mut record.field_sources,
            work_center_kind,
            observation,
        );
    }

    let work_unit_value = observation
        .work_unit
        .as_ref()
        .or(observation.process.as_ref());
    merge_field(
        &mut record.work_unit,
        &mut record.field_sources,
        "work_unit",
        work_unit_value,
        observation,
    );
    merge_field(
        &mut record.process,
        &mut record.field_sources,
        "process",
        work_unit_value,
        observation,
    );
    merge_field(
        &mut record.function,
        &mut record.field_sources,
        "function",
        observation.function.as_ref(),
        observation,
    );
    merge_hardware_identities(
        &mut record.hardware_identities,
        &mut record.field_sources,
        observation.hardware_identities.as_deref(),
        observation,
    );
    merge_application_identities(
        &mut record.application_identities,
        &mut record.field_sources,
        observation.application_identities.as_deref(),
        observation,
    );
    merge_aliases(
        &mut record.aliases,
        &mut record.field_sources,
        observation.aliases.as_deref(),
        observation,
    );
    merge_relations(
        &mut record.relations,
        &mut record.field_sources,
        observation.relations.as_deref(),
        observation,
    );

    if let Some(protocols) = &observation.protocols {
        let should_replace = record
            .field_sources
            .get("protocols")
            .map(|field| source_rank(observation.source) <= source_rank(field.source))
            .unwrap_or(true);
        if should_replace {
            record.protocols = protocols.clone();
            record.field_sources.insert(
                "protocols".to_string(),
                crate::MetadataField {
                    value: protocols.join(","),
                    source: observation.source,
                    updated_at: observation.observed_at,
                },
            );
        }
    }

    if let Some(status) = observation.status {
        let should_replace = record
            .field_sources
            .get("status")
            .map(|field| source_rank(observation.source) <= source_rank(field.source))
            .unwrap_or(true);
        if should_replace {
            record.status = status;
            record.field_sources.insert(
                "status".to_string(),
                crate::MetadataField {
                    value: format!("{status:?}"),
                    source: observation.source,
                    updated_at: observation.observed_at,
                },
            );
        }
    }

    synchronize_alias_pair(
        &mut record.site,
        &mut record.facility,
        &mut record.field_sources,
        "site",
        "facility",
    );
    synchronize_alias_pair(
        &mut record.area,
        &mut record.zone,
        &mut record.field_sources,
        "area",
        "zone",
    );
    synchronize_alias_pair(
        &mut record.work_center,
        &mut record.cell,
        &mut record.field_sources,
        "work_center",
        "cell",
    );
    synchronize_alias_pair(
        &mut record.work_unit,
        &mut record.process,
        &mut record.field_sources,
        "work_unit",
        "process",
    );

    record.updated_at = Utc::now();
    record.fqdn = build_semantic_name(&record.naming_segments(), sibling_names, zone_suffix);
    Ok(record)
}

fn merge_field(
    target: &mut Option<String>,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    field_name: &str,
    incoming: Option<&String>,
    observation: &Observation,
) {
    let Some(value) = incoming else {
        return;
    };

    let should_replace = field_sources
        .get(field_name)
        .map(|field| source_rank(observation.source) <= source_rank(field.source))
        .unwrap_or(true);

    if should_replace {
        *target = Some(value.clone());
        field_sources.insert(
            field_name.to_string(),
            MetadataField {
                value: value.clone(),
                source: observation.source,
                updated_at: observation.observed_at,
            },
        );
    }
}

fn merge_work_center_kind(
    target: &mut Option<Isa95WorkCenterKind>,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    incoming: Isa95WorkCenterKind,
    observation: &Observation,
) {
    let should_replace = field_sources
        .get("work_center_kind")
        .map(|field| source_rank(observation.source) <= source_rank(field.source))
        .unwrap_or(true);

    if should_replace {
        *target = Some(incoming);
        field_sources.insert(
            "work_center_kind".to_string(),
            MetadataField {
                value: incoming.as_str().to_string(),
                source: observation.source,
                updated_at: observation.observed_at,
            },
        );
    }
}

fn merge_node_kind(
    target: &mut Isa95NodeKind,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    incoming: Isa95NodeKind,
    observation: &Observation,
) {
    let should_replace = field_sources
        .get("node_kind")
        .map(|field| source_rank(observation.source) <= source_rank(field.source))
        .unwrap_or(true);

    if should_replace {
        *target = incoming;
        field_sources.insert(
            "node_kind".to_string(),
            MetadataField {
                value: incoming.as_str().to_string(),
                source: observation.source,
                updated_at: observation.observed_at,
            },
        );
    }
}

fn synchronize_alias_pair(
    canonical: &mut Option<String>,
    alias: &mut Option<String>,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    canonical_key: &str,
    alias_key: &str,
) {
    let selected = canonical.clone().or(alias.clone());
    let metadata = field_sources
        .get(canonical_key)
        .cloned()
        .or_else(|| field_sources.get(alias_key).cloned());

    if let Some(value) = selected {
        *canonical = Some(value.clone());
        *alias = Some(value.clone());

        if let Some(mut metadata) = metadata {
            metadata.value = value.clone();
            field_sources.insert(canonical_key.to_string(), metadata.clone());
            field_sources.insert(alias_key.to_string(), metadata);
        }
    }
}

fn merge_application_identities(
    target: &mut Vec<ApplicationIdentity>,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    incoming: Option<&[ApplicationIdentity]>,
    observation: &Observation,
) {
    let Some(incoming) = incoming else {
        return;
    };

    let should_replace = field_sources
        .get("application_identities")
        .map(|field| source_rank(observation.source) <= source_rank(field.source))
        .unwrap_or(true);

    if should_replace {
        for identity in incoming.iter().map(normalize_application_identity) {
            if !target.iter().any(|existing| existing.kind == identity.kind && existing.value.eq_ignore_ascii_case(&identity.value)) {
                target.push(identity);
            }
        }
        target.sort_by(|left, right| left.value.cmp(&right.value));
        field_sources.insert(
            "application_identities".to_string(),
            MetadataField {
                value: target
                    .iter()
                    .map(|identity| format!("{}={}", identity.kind.as_str(), identity.value))
                    .collect::<Vec<_>>()
                    .join(","),
                source: observation.source,
                updated_at: observation.observed_at,
            },
        );
    }
}

fn merge_hardware_identities(
    target: &mut Vec<HardwareIdentity>,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    incoming: Option<&[HardwareIdentity]>,
    observation: &Observation,
) {
    let should_replace = field_sources
        .get("hardware_identities")
        .map(|field| source_rank(observation.source) <= source_rank(field.source))
        .unwrap_or(true);

    if should_replace {
        let mut merged = target.clone();
        if let Some(incoming) = incoming {
            for identity in incoming.iter().map(normalize_hardware_identity) {
                if !merged.iter().any(|existing| {
                    existing.kind == identity.kind
                        && existing.value.eq_ignore_ascii_case(&identity.value)
                }) {
                    merged.push(identity);
                }
            }
        }
        if let Some(mac) = observation.mac.as_ref() {
            let identity = normalize_hardware_identity(&HardwareIdentity {
                kind: HardwareIdentityKind::MacAddress,
                value: mac.clone(),
                label: Some("mac".to_string()),
            });
            if !identity.value.is_empty()
                && !merged.iter().any(|existing| {
                    existing.kind == identity.kind
                        && existing.value.eq_ignore_ascii_case(&identity.value)
                })
            {
                merged.push(identity);
            }
        }
        merged.sort_by(|left, right| left.value.cmp(&right.value));
        *target = merged;
        field_sources.insert(
            "hardware_identities".to_string(),
            MetadataField {
                value: target
                    .iter()
                    .map(|identity| format!("{}={}", identity.kind.as_str(), identity.value))
                    .collect::<Vec<_>>()
                    .join(","),
                source: observation.source,
                updated_at: observation.observed_at,
            },
        );
    }
}

fn merge_aliases(
    target: &mut Vec<String>,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    incoming: Option<&[String]>,
    observation: &Observation,
) {
    let Some(incoming) = incoming else {
        return;
    };

    let should_replace = field_sources
        .get("aliases")
        .map(|field| source_rank(observation.source) <= source_rank(field.source))
        .unwrap_or(true);

    if should_replace {
        for alias in incoming.iter().map(|alias| normalize_alias(alias)).filter(|alias| !alias.is_empty()) {
            if !target.iter().any(|existing| existing.eq_ignore_ascii_case(&alias)) {
                target.push(alias);
            }
        }
        target.sort();
        field_sources.insert(
            "aliases".to_string(),
            MetadataField {
                value: target.join(","),
                source: observation.source,
                updated_at: observation.observed_at,
            },
        );
    }
}

fn merge_relations(
    target: &mut Vec<SemanticRelation>,
    field_sources: &mut std::collections::BTreeMap<String, MetadataField>,
    incoming: Option<&[SemanticRelation]>,
    observation: &Observation,
) {
    let Some(incoming) = incoming else {
        return;
    };

    let should_replace = field_sources
        .get("relations")
        .map(|field| source_rank(observation.source) <= source_rank(field.source))
        .unwrap_or(true);

    if should_replace {
        for relation in incoming
            .iter()
            .map(normalize_relation)
            .filter(|relation| !relation.relation.is_empty() && !relation.target.is_empty())
        {
            if !target.iter().any(|existing| {
                existing.relation.eq_ignore_ascii_case(&relation.relation)
                    && existing.target.eq_ignore_ascii_case(&relation.target)
            }) {
                target.push(relation);
            }
        }
        target.sort_by(|left, right| {
            left.relation
                .cmp(&right.relation)
                .then(left.target.cmp(&right.target))
        });
        field_sources.insert(
            "relations".to_string(),
            MetadataField {
                value: target
                    .iter()
                    .map(|relation| format!("{}->{}", relation.relation, relation.target))
                    .collect::<Vec<_>>()
                    .join(","),
                source: observation.source,
                updated_at: observation.observed_at,
            },
        );
    }
}

fn source_rank(source: ObservationSource) -> u8 {
    match source {
        ObservationSource::ManualApi => 0,
        ObservationSource::ProtocolAnalysis => 1,
        ObservationSource::SwitchIntelligence => 2,
        ObservationSource::DhcpFingerprint => 3,
        ObservationSource::ReplacementInference => 4,
        ObservationSource::Discovery => 5,
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sdns_common::{DeviceId, ObservationId};

    use crate::{
        ApplicationIdentity, ApplicationIdentityKind, Isa95NodeKind, Observation,
        ObservationSource, SemanticRelation,
    };

    use super::merge_observation;

    #[test]
    fn protocol_observation_overrides_dhcp_model() {
        let device_id = DeviceId::new();
        let dhcp = Observation {
            id: ObservationId::new(),
            device_id,
            observed_at: Utc::now(),
            source: ObservationSource::DhcpFingerprint,
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
            work_center_kind: Some(crate::Isa95WorkCenterKind::ProcessCell),
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
            status: None,
        };

        let initial = merge_observation(None, &dhcp, &[], "local").expect("initial merge");
        let probe = Observation {
            model: Some("PowerFlex525".to_string()),
            source: ObservationSource::ProtocolAnalysis,
            id: ObservationId::new(),
            ..dhcp
        };

        let merged = merge_observation(Some(initial), &probe, &[], "local").expect("merged");
        assert_eq!(merged.model.as_deref(), Some("PowerFlex525"));
    }

    #[test]
    fn syncs_legacy_aliases_from_isa95_fields() {
        let record = merge_observation(
            None,
            &Observation {
                id: ObservationId::new(),
                device_id: DeviceId::new(),
                observed_at: Utc::now(),
                source: ObservationSource::ManualApi,
                node_kind: Isa95NodeKind::Device,
                external_ip: None,
                internal_ip: None,
                class: Some("sensor".to_string()),
                vendor: None,
                model: None,
                protocols: None,
                mac: None,
                switch_port: None,
                enterprise: None,
                site: Some("Austin".to_string()),
                area: Some("Assembly".to_string()),
                work_center: Some("Line1".to_string()),
                work_center_kind: Some(crate::Isa95WorkCenterKind::ProductionLine),
                work_unit: Some("Station7".to_string()),
                facility: None,
                zone: None,
                cell: None,
                process: None,
                function: Some("TorqueSensor".to_string()),
                hardware_identities: Some(vec![crate::HardwareIdentity {
                    kind: crate::HardwareIdentityKind::SerialNumber,
                    value: "SN-TORQUE-0007".to_string(),
                    label: Some("serial".to_string()),
                }]),
                application_identities: Some(vec![ApplicationIdentity {
                    kind: ApplicationIdentityKind::Urn,
                    value: "urn:mes:asset:torque-sensor-7".to_string(),
                    label: Some("mes".to_string()),
                }]),
                aliases: Some(vec!["station7-torque".to_string()]),
                relations: Some(vec![SemanticRelation {
                    relation: "reports-to".to_string(),
                    target: "urn:mes:station:7".to_string(),
                    label: None,
                }]),
                status: None,
            },
            &[],
            "local",
        )
        .expect("record");

        assert_eq!(record.facility.as_deref(), Some("Austin"));
        assert_eq!(record.zone.as_deref(), Some("Assembly"));
        assert_eq!(record.cell.as_deref(), Some("Line1"));
        assert_eq!(record.process.as_deref(), Some("Station7"));
        assert_eq!(record.application_identities.len(), 1);
        assert_eq!(record.aliases, vec!["station7-torque".to_string()]);
        assert_eq!(
            record.fqdn,
            "TorqueSensor.Station7.Line1.Assembly.Austin.local"
        );
    }
}
