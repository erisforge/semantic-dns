use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    extract::{
        FromRef, FromRequestParts, Path, Query, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sdns_audit::{AuditEvent, AuditEventRecord, AuditLedger};
use sdns_bind::DnsPublisher;
use sdns_common::{AppError, Permission, Principal};
use sdns_core::{Observation, RecordFilter};
use sdns_dhcp::{
    AuthorizeQuarantineRequest, FingerprintClassification, FingerprintInput, FingerprintRule,
    RoleMatch, RoleTemplate, choose_assignment, match_rule,
};
use sdns_fathom::FathomImportReport;
use sdns_store::SemanticStore;
use tokio::sync::broadcast;

type ApiResult<T> = Result<T, ApiError>;

#[derive(Clone)]
pub struct ApiState {
    pub store: Arc<dyn SemanticStore>,
    pub audit: AuditLedger,
    pub dns: Arc<dyn DnsPublisher>,
    pub tokens: Arc<HashMap<String, Principal>>,
    pub events: broadcast::Sender<String>,
    pub fathom_database_url: Option<String>,
}

impl FromRef<ApiState> for Arc<HashMap<String, Principal>> {
    fn from_ref(input: &ApiState) -> Self {
        Arc::clone(&input.tokens)
    }
}

#[derive(Clone)]
pub struct AuthenticatedPrincipal(pub Principal);

impl<S> FromRequestParts<S> for AuthenticatedPrincipal
where
    Arc<HashMap<String, Principal>>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let tokens = Arc::<HashMap<String, Principal>>::from_ref(state);
        let raw_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                ApiError(AppError::Unauthenticated(
                    "missing bearer token".to_string(),
                ))
            })?;
        let token = raw_header.strip_prefix("Bearer ").ok_or_else(|| {
            ApiError(AppError::Unauthenticated(
                "invalid authorization scheme".to_string(),
            ))
        })?;
        let principal = tokens.get(token).cloned().ok_or_else(|| {
            ApiError(AppError::Unauthenticated(
                "unknown bearer token".to_string(),
            ))
        })?;
        Ok(Self(principal))
    }
}

#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({
            "error": {
                "message": self.0.to_string(),
            }
        }));
        (self.0.status_code(), body).into_response()
    }
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/resolve/{target}", get(resolve))
        .route("/api/v1/dns/query", get(query_records))
        .route("/api/v1/observations", post(ingest_observation))
        .route("/api/v1/dhcp/leases", get(list_leases))
        .route("/api/v1/dhcp/quarantine", get(list_quarantine))
        .route(
            "/api/v1/dhcp/quarantine/authorize",
            post(authorize_quarantine),
        )
        .route("/api/v1/dhcp/dns/sync-status", get(sync_status))
        .route("/api/v1/dhcp/dns/reconcile", post(reconcile))
        .route(
            "/api/v1/dhcp/fingerprints",
            get(list_fingerprints).post(put_fingerprint),
        )
        .route(
            "/api/v1/dhcp/templates",
            get(list_templates).post(put_template),
        )
        .route("/api/v1/dhcp/evaluate", post(evaluate_request))
        .route("/api/v1/integrations/fathom/import", post(import_fathom))
        .route("/api/v1/audit/events", get(list_audit_events))
        .route("/api/v1/ws", get(ws_updates))
        .with_state(state)
}

async fn health(State(state): State<ApiState>) -> ApiResult<Json<serde_json::Value>> {
    let sync = state.store.sync_status().await?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "sync_status": sync,
    })))
}

async fn resolve(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Path(target): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    principal.require(Permission::Read)?;
    let record = state
        .store
        .resolve(&target)
        .await?
        .ok_or_else(|| AppError::NotFound(target.clone()))?;
    Ok(Json(serde_json::json!(record)))
}

async fn query_records(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Query(filter): Query<RecordFilter>,
) -> ApiResult<Json<Vec<sdns_core::SemanticRecord>>> {
    principal.require(Permission::Read)?;
    Ok(Json(state.store.query(filter).await?))
}

async fn ingest_observation(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Json(observation): Json<Observation>,
) -> ApiResult<Json<sdns_core::SemanticRecord>> {
    principal.require(Permission::SystemIngest)?;
    let record = state.store.upsert_observation(observation.clone()).await?;
    state.dns.publish(&record).await?;
    state
        .audit
        .append(AuditEvent {
            event_type: "observation.ingested".to_string(),
            payload: serde_json::json!({
                "device_id": record.device_id,
                "fqdn": record.fqdn,
                "source": observation.source,
            }),
            created_at: chrono::Utc::now(),
        })
        .await
        .map_err(|err| AppError::Internal(format!("audit append failed: {err}")))?;
    let event = serde_json::to_string(&record)
        .map_err(|err| AppError::Internal(format!("event serialization failed: {err}")))?;
    let _ = state.events.send(event);
    Ok(Json(record))
}

async fn list_leases(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<sdns_dhcp::DhcpLease>>> {
    principal.require(Permission::Read)?;
    Ok(Json(state.store.list_leases().await?))
}

async fn list_quarantine(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<sdns_dhcp::QuarantineEntry>>> {
    principal.require(Permission::DhcpWrite)?;
    Ok(Json(state.store.list_quarantine().await?))
}

async fn authorize_quarantine(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Json(request): Json<AuthorizeQuarantineRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    principal.require(Permission::DhcpWrite)?;
    let removed = state.store.authorize_quarantine(request.clone()).await?;
    state
        .audit
        .append(AuditEvent {
            event_type: "dhcp.quarantine.authorized".to_string(),
            payload: serde_json::json!({
                "mac": request.mac,
                "assigned_role": request.assigned_role,
                "operator": request.operator,
                "justification": request.justification,
                "removed": removed.is_some(),
            }),
            created_at: chrono::Utc::now(),
        })
        .await
        .map_err(|err| AppError::Internal(format!("audit append failed: {err}")))?;
    Ok(Json(serde_json::json!({ "removed": removed })))
}

async fn sync_status(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
) -> ApiResult<Json<sdns_core::SyncStatus>> {
    principal.require(Permission::Read)?;
    Ok(Json(state.store.sync_status().await?))
}

async fn reconcile(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
) -> ApiResult<Json<sdns_core::SyncStatus>> {
    principal.require(Permission::DnsAdmin)?;
    Ok(Json(state.store.mark_reconciliation().await?))
}

async fn list_fingerprints(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<FingerprintRule>>> {
    principal.require(Permission::Read)?;
    Ok(Json(state.store.list_fingerprints().await?))
}

async fn put_fingerprint(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Json(rule): Json<FingerprintRule>,
) -> ApiResult<StatusCode> {
    principal.require(Permission::DhcpWrite)?;
    state.store.put_fingerprint(rule).await?;
    Ok(StatusCode::CREATED)
}

async fn list_templates(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
) -> ApiResult<Json<Vec<RoleTemplate>>> {
    principal.require(Permission::Read)?;
    Ok(Json(state.store.list_templates().await?))
}

async fn put_template(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Json(template): Json<RoleTemplate>,
) -> ApiResult<StatusCode> {
    principal.require(Permission::DhcpWrite)?;
    state.store.put_template(template).await?;
    Ok(StatusCode::CREATED)
}

#[derive(Debug, Clone, serde::Deserialize)]
struct EvaluateRequest {
    input: FingerprintInput,
    function_hint: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct EvaluateResponse {
    classifications: Vec<FingerprintClassification>,
    best_role: Option<RoleMatch>,
}

async fn evaluate_request(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Json(request): Json<EvaluateRequest>,
) -> ApiResult<Json<EvaluateResponse>> {
    principal.require(Permission::DhcpWrite)?;
    let rules = state.store.list_fingerprints().await?;
    let templates = state.store.list_templates().await?;
    let classifications = rules
        .iter()
        .filter_map(|rule| match_rule(&request.input, rule))
        .collect::<Vec<_>>();
    let best_role = classifications.first().and_then(|classification| {
        templates.first().and_then(|template| {
            choose_assignment(
                classification,
                request.function_hint.as_deref(),
                template,
                &[],
            )
        })
    });
    Ok(Json(EvaluateResponse {
        classifications,
        best_role,
    }))
}

async fn import_fathom(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
) -> ApiResult<Json<FathomImportReport>> {
    principal.require(Permission::DnsAdmin)?;
    let database_url = state
        .fathom_database_url
        .as_deref()
        .ok_or_else(|| AppError::Validation("fathom.database_url is not configured".to_string()))?;
    let report = sdns_fathom::import_from_fathom(database_url, state.store.as_ref()).await?;
    state
        .audit
        .append(AuditEvent {
            event_type: "fathom.import.completed".to_string(),
            payload: serde_json::json!(report),
            created_at: chrono::Utc::now(),
        })
        .await
        .map_err(|err| AppError::Internal(format!("audit append failed: {err}")))?;
    Ok(Json(report))
}

#[derive(Debug, Clone, serde::Deserialize)]
struct AuditQuery {
    limit: Option<usize>,
}

async fn list_audit_events(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    Query(query): Query<AuditQuery>,
) -> ApiResult<Json<Vec<AuditEventRecord>>> {
    principal.require(Permission::Read)?;
    let limit = query.limit.unwrap_or(100).min(500) as i64;
    let events = state
        .audit
        .list_recent(limit)
        .await
        .map_err(|err| AppError::Internal(format!("audit list failed: {err}")))?;
    Ok(Json(events))
}

async fn ws_updates(
    AuthenticatedPrincipal(principal): AuthenticatedPrincipal,
    State(state): State<ApiState>,
    ws: WebSocketUpgrade,
) -> ApiResult<impl IntoResponse> {
    principal.require(Permission::Read)?;
    let receiver = state.events.subscribe();
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, receiver)))
}

async fn handle_socket(mut socket: WebSocket, mut receiver: broadcast::Receiver<String>) {
    while let Ok(message) = receiver.recv().await {
        if socket.send(Message::Text(message.into())).await.is_err() {
            break;
        }
    }
}
