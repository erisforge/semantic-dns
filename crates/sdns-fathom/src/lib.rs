use chrono::{DateTime, Utc};
use sdns_common::{AppError, DeviceId, LeaseId, ObservationId};
use sdns_core::{Isa95NodeKind, Observation, ObservationSource, RecordStatus};
use sdns_dhcp::DhcpLease;
use sdns_store::SemanticStore;
use sqlx_core::query_as::query_as;
use sqlx_postgres::{PgPoolOptions, Postgres};

type FathomAssetRow = (
    uuid::Uuid,
    DateTime<Utc>,
    DateTime<Utc>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

#[derive(Debug, Clone, serde::Serialize)]
pub struct FathomImportReport {
    pub imported_assets: usize,
    pub imported_leases: usize,
    pub skipped_assets: usize,
}

pub async fn import_from_fathom(
    database_url: &str,
    store: &dyn SemanticStore,
) -> Result<FathomImportReport, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
        .map_err(|err| AppError::Internal(format!("fathom postgres connect failed: {err}")))?;

    let assets: Vec<FathomAssetRow> = query_as::<Postgres, _>(
        "SELECT id, first_seen, last_seen, device_type, vendor, model, firmware, hostname
         FROM assets
         ORDER BY last_seen DESC",
    )
    .fetch_all(&pool)
    .await
    .map_err(|err| AppError::Internal(format!("fathom asset query failed: {err}")))?;

    let mut report = FathomImportReport {
        imported_assets: 0,
        imported_leases: 0,
        skipped_assets: 0,
    };

    for (asset_id, first_seen, last_seen, device_type, vendor, model, firmware, hostname) in assets
    {
        let interfaces: Vec<(String, Vec<String>, Option<i16>)> = query_as::<Postgres, _>(
            "SELECT mac_address, ip_addresses, vlan_id
             FROM asset_interfaces
             WHERE asset_id = $1",
        )
        .bind(asset_id)
        .fetch_all(&pool)
        .await
        .map_err(|err| AppError::Internal(format!("fathom interface query failed: {err}")))?;

        let Some((mac, ip_addresses, _vlan_id)) = interfaces.first() else {
            report.skipped_assets += 1;
            continue;
        };

        let internal_ip = ip_addresses.first().cloned();
        let source = if model.is_some() || vendor.is_some() || firmware.is_some() {
            ObservationSource::ProtocolAnalysis
        } else {
            ObservationSource::Discovery
        };
        let observation = Observation {
            id: ObservationId::new(),
            device_id: DeviceId::from(asset_id),
            observed_at: last_seen,
            source,
            node_kind: Isa95NodeKind::Device,
            external_ip: None,
            internal_ip: internal_ip.clone(),
            class: device_type.clone(),
            vendor: vendor.clone(),
            model: model.clone().or(firmware.clone()),
            protocols: None,
            mac: Some(mac.clone()),
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
            function: hostname.clone(),
            hardware_identities: None,
            application_identities: None,
            aliases: None,
            relations: None,
            status: Some(RecordStatus::Active),
        };

        let record = store.upsert_observation(observation).await?;
        report.imported_assets += 1;

        if let Some(internal_ip) = internal_ip {
            store
                .upsert_lease(DhcpLease {
                    id: LeaseId::new(),
                    mac: mac.clone(),
                    address: internal_ip,
                    external_ip: record.external_ip.clone(),
                    dns_name: Some(record.fqdn.clone()),
                    role: None,
                    class: device_type.clone(),
                    switch_port: None,
                    status: RecordStatus::Active,
                    first_seen,
                    last_seen,
                })
                .await?;
            report.imported_leases += 1;
        }
    }

    Ok(report)
}
