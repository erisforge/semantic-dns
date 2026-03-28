use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use clap::Parser;
use sdns_api::{ApiState, router};
use sdns_audit::AuditLedger;
use sdns_bind::{DnsPublisher, FileDnsPublisher};
use sdns_common::{ApiTokenConfig, AppConfig, Principal, Role, TemplateId};
use sdns_core::Isa95WorkCenterKind;
use sdns_dhcp::{FingerprintClassification, FingerprintRule, RoleAssignment, RoleTemplate};
use sdns_store::{PostgresStore, SemanticStore};
use tokio::sync::broadcast;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(long, env = "SDNS_CONFIG")]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();
    let config = AppConfig::load(cli.config.as_deref())?;
    let audit = AuditLedger::open(&config.audit.database_url, &config.audit.schema).await?;
    let store: Arc<dyn SemanticStore> =
        Arc::new(PostgresStore::open(&config.store.database_url, &config.store.schema).await?);
    seed_reference_data(Arc::clone(&store)).await?;
    let dns_publisher = FileDnsPublisher::new(
        config.dns.zone.clone(),
        config.dns.zone_file.clone(),
        Arc::clone(&store),
    );
    dns_publisher.sync_all().await?;
    let dns: Arc<dyn DnsPublisher> = Arc::new(dns_publisher);
    let (events, _) = broadcast::channel(128);
    let tokens = Arc::new(build_tokens(&config.api_tokens));
    let state = ApiState {
        store,
        audit,
        dns,
        tokens,
        events,
        fathom_database_url: config
            .fathom
            .database_url
            .clone()
            .filter(|value| !value.trim().is_empty()),
    };

    let app = router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());
    let addr: SocketAddr = config.http.bind.parse()?;
    info!(%addr, "semantic-dns listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_tokens(configs: &[ApiTokenConfig]) -> HashMap<String, Principal> {
    configs
        .iter()
        .map(|config| {
            (
                config.token.clone(),
                Principal {
                    id: config.principal_id,
                    name: config.name.clone(),
                    role: config.role,
                },
            )
        })
        .collect()
}

async fn seed_reference_data(store: Arc<dyn SemanticStore>) -> anyhow::Result<()> {
    if store.list_fingerprints().await?.is_empty() {
        store
            .put_fingerprint(FingerprintRule {
                id: sdns_common::FingerprintId::new(),
                name: "rockwell-powerflex-525".to_string(),
                mac_oui: Some("00:00:BC".to_string()),
                option_60_glob: Some("Rockwell*".to_string()),
                option_55_order: Some(vec![1, 3, 6, 15, 28, 42]),
                classification: FingerprintClassification {
                    vendor: "rockwell".to_string(),
                    class: "vfd".to_string(),
                    model_family: "PowerFlex500".to_string(),
                    confidence: 0.8,
                    protocols: vec!["ethernet-ip".to_string()],
                },
            })
            .await?;
    }

    if store.list_templates().await?.is_empty() {
        store
            .put_template(RoleTemplate {
                id: TemplateId::new(),
                name: "cell5-default".to_string(),
                site_id: Some("Milwaukee".to_string()),
                area_id: Some("Zone3".to_string()),
                work_center_id: Some("Cell5".to_string()),
                work_center_kind: Some(Isa95WorkCenterKind::ProcessCell),
                cell_id: "Cell5".to_string(),
                zone_suffix: "local".to_string(),
                assignments: vec![RoleAssignment {
                    role: "conveyor-vfd".to_string(),
                    address: "192.168.1.47".to_string(),
                    class_match: Some("vfd".to_string()),
                    vendor_match: Some("rockwell".to_string()),
                    function_match: Some("conveyor".to_string()),
                    work_unit_id: Some("Conveyor".to_string()),
                    process_area: "Conveyor".to_string(),
                }],
                unassigned_range: vec!["192.168.1.200".to_string()],
                quarantine_range: vec!["192.168.1.241".to_string()],
            })
            .await?;
    }

    let _ = Role::Admin;
    Ok(())
}
