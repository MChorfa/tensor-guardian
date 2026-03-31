//! REST HTTP handlers for accelerator API

use crate::{
    convert::{parse_accelerator_id, accelerator_to_proto, metric_to_proto, sample_to_proto_vec},
    error::{ApiError, Result},
    proto::MetricSample,
};
use axum::{
    extract::{Path, Query, State},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use tensor_guardian_domain::{
    ports::AcceleratorBackend,
    PlatformBackendFactory,
};

/// API state shared across handlers
#[derive(Clone)]
pub struct ApiState {
    pub backends: Arc<Vec<Arc<dyn AcceleratorBackend>>>,
}

impl ApiState {
    /// Create new state with auto-detected backends
    pub async fn new() -> anyhow::Result<Self> {
        let backends = PlatformBackendFactory::create_backends().await;
        info!("REST API initialized with {} backend(s)", backends.len());
        
        Ok(Self {
            backends: Arc::new(backends),
        })
    }
}

/// Query parameters for listing accelerators
#[derive(Debug, Deserialize)]
pub struct ListParams {
    /// Filter by backend type
    pub backend: Option<String>,
    /// Filter by accelerator type
    pub r#type: Option<String>,
}

/// Create router with all routes
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/accelerators", get(list_accelerators))
        .route("/accelerators/:id", get(get_accelerator))
        .route("/accelerators/:id/metrics", get(get_metrics).post(collect_metrics_post))
        .route("/health", get(health_check))
        .route("/version", get(version))
        .with_state(state)
}

/// Accelerator response for REST API
#[derive(Serialize)]
pub struct AcceleratorResponse {
    pub id: String,
    pub name: String,
    pub accelerator_type: String,
    pub vendor: String,
    pub model: String,
    pub backend_type: String,
    pub sensors: Vec<SensorResponse>,
    pub pci_bus_id: String,
    pub labels: std::collections::HashMap<String, String>,
}

/// Sensor response for REST API
#[derive(Serialize)]
pub struct SensorResponse {
    pub id: String,
    pub name: String,
    pub metric_type: String,
    pub unit: String,
}

/// List all accelerators
async fn list_accelerators(
    State(state): State<ApiState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<AcceleratorResponse>>> {
    debug!("REST: list_accelerators, params={:?}", params);

    let mut accelerators = Vec::new();

    for backend in state.backends.iter() {
        let backend_type = backend.backend_type();

        // Apply backend filter
        if let Some(ref filter) = params.backend {
            if backend_type != filter {
                continue;
            }
        }

        match backend.discover().await {
            Ok(accs) => {
                for acc in accs {
                    // Apply type filter
                    if let Some(ref type_filter) = params.r#type {
                        if format!("{:?}", acc.accelerator_type) != *type_filter {
                            continue;
                        }
                    }
                    accelerators.push(accelerator_to_rest(&acc));
                }
            }
            Err(e) => {
                warn!("Backend {} discovery failed: {}", backend_type, e);
            }
        }
    }

    info!("REST: list_accelerators returned {} accelerators", accelerators.len());
    Ok(Json(accelerators))
}

/// Convert accelerator to REST response
fn accelerator_to_rest(accel: &tensor_guardian_domain::aggregates::Accelerator) -> AcceleratorResponse {
    AcceleratorResponse {
        id: accel.id.to_string(),
        name: accel.name.clone(),
        accelerator_type: format!("{:?}", accel.accelerator_type),
        vendor: accel.vendor.clone(),
        model: accel.model.clone(),
        backend_type: accel.backend_type.clone(),
        sensors: accel.sensors.values().map(sensor_to_rest).collect(),
        pci_bus_id: accel.pci_info.as_ref().map(|p| p.bus_id.clone()).unwrap_or_default(),
        labels: accel.labels.inner().clone(),
    }
}

/// Convert sensor to REST response
fn sensor_to_rest(sensor: &tensor_guardian_domain::entities::Sensor) -> SensorResponse {
    SensorResponse {
        id: sensor.id.to_string(),
        name: sensor.name.clone(),
        metric_type: format!("{:?}", sensor.metric_type),
        unit: format!("{:?}", sensor.unit),
    }
}

/// Get single accelerator by ID
async fn get_accelerator(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<AcceleratorResponse>> {
    debug!("REST: get_accelerator, id={}", id);

    for backend in state.backends.iter() {
        match backend.discover().await {
            Ok(accs) => {
                if let Some(acc) = accs.iter().find(|a| a.id.to_string() == id) {
                    return Ok(Json(accelerator_to_rest(acc)));
                }
            }
            Err(e) => {
                warn!("Backend {} discovery failed: {}", backend.backend_type(), e);
            }
        }
    }

    Err(ApiError::not_found(format!("Accelerator not found: {}", id)))
}

/// Query parameters for metrics
#[derive(Debug, Deserialize)]
pub struct MetricsParams {
    /// Sensor filter (comma-separated)
    pub sensors: Option<String>,
}

/// Metric sample response for REST API
#[derive(Serialize)]
pub struct MetricSampleResponse {
    pub accelerator_id: String,
    pub sensor_id: String,
    pub metric_name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp_ns: i64,
    pub labels: std::collections::HashMap<String, String>,
}

/// Collect metrics response
#[derive(Serialize)]
pub struct CollectMetricsResponse {
    pub accelerator_id: String,
    pub timestamp_ns: i64,
    pub samples: Vec<MetricSampleResponse>,
}

/// Get cached metrics (placeholder - returns latest collected)
async fn get_metrics(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Query(params): Query<MetricsParams>,
) -> Result<Json<CollectMetricsResponse>> {
    debug!("REST: get_metrics, id={}, sensors={:?}", id, params.sensors);

    // For now, trigger fresh collection
    collect_metrics(State(state), Path(id)).await
}

/// POST version for collecting metrics
async fn collect_metrics_post(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<CollectMetricsResponse>> {
    debug!("REST: collect_metrics_post, id={}", id);
    collect_metrics(State(state), Path(id)).await
}

/// Trigger metric collection
async fn collect_metrics(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<CollectMetricsResponse>> {
    debug!("REST: collect_metrics, id={}", id);

    let accel_id = parse_accelerator_id(&id)
        .map_err(ApiError::invalid_request)?;

    // Find accelerator and its backend
    let mut target_backend = None;
    let mut target_accelerator = None;

    for backend in state.backends.iter() {
        match backend.discover().await {
            Ok(accs) => {
                if let Some(acc) = accs.into_iter().find(|a| a.id == accel_id) {
                    target_backend = Some(backend.clone());
                    target_accelerator = Some(acc);
                    break;
                }
            }
            Err(e) => {
                warn!("Backend {} discovery failed: {}", backend.backend_type(), e);
            }
        }
    }

    let (backend, accelerator) = target_backend
        .zip(target_accelerator)
        .ok_or_else(|| ApiError::not_found(format!("Accelerator not found: {}", id)))?;

    // Collect metrics
    let sample = backend
        .collect(&accelerator)
        .await
        .map_err(ApiError::from)?;

    let timestamp_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    let samples: Vec<MetricSampleResponse> = sample.metrics.iter()
        .map(|m| MetricSampleResponse {
            accelerator_id: m.accelerator_id.to_string(),
            sensor_id: m.sensor_id.to_string(),
            metric_name: format!("{:?}", m.metric_type),
            value: crate::convert::metric_value_to_f64(&m.value),
            unit: format!("{:?}", m.unit),
            timestamp_ns: m.timestamp.nanos,
            labels: m.labels.inner().clone(),
        })
        .collect();

    info!("REST: collect_metrics returned {} samples", samples.len());

    Ok(Json(CollectMetricsResponse {
        accelerator_id: id,
        timestamp_ns,
        samples,
    }))
}

/// Health check response (REST version without protobuf)
#[derive(Serialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub backends: Vec<BackendStatusRest>,
    pub version: String,
}

/// REST version of backend status
#[derive(Serialize)]
pub struct BackendStatusRest {
    pub backend_type: String,
    pub healthy: bool,
    pub version: String,
    pub status_message: String,
    pub accelerator_count: i32,
}

/// Health check endpoint
async fn health_check(State(state): State<ApiState>) -> Result<Json<HealthResponse>> {
    debug!("REST: health_check");

    use tensor_guardian_domain::ports::BackendHealth;

    let mut all_healthy = true;
    let mut backend_statuses = Vec::new();

    for backend in state.backends.iter() {
        let backend_type = backend.backend_type();
        let health = backend.health().await;
        let version = backend.version();

        let accelerator_count = backend.discover().await.map(|a| a.len() as i32).unwrap_or(0);

        let (healthy, status_message) = match health {
            BackendHealth::Healthy => (true, "OK".to_string()),
            BackendHealth::Degraded { reason } => {
                all_healthy = false;
                (true, format!("Degraded: {}", reason))
            }
            BackendHealth::Unhealthy { reason } => {
                all_healthy = false;
                (false, format!("Unhealthy: {}", reason))
            }
        };

        backend_statuses.push(BackendStatusRest {
            backend_type: backend_type.to_string(),
            healthy,
            version: version.to_string(),
            status_message,
            accelerator_count,
        });
    }

    if state.backends.is_empty() {
        all_healthy = false;
    }

    Ok(Json(HealthResponse {
        healthy: all_healthy,
        backends: backend_statuses,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Version response
#[derive(Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub name: String,
}

/// Version endpoint
async fn version() -> Json<VersionResponse> {
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
        name: "tensor-guardian".to_string(),
    })
}
