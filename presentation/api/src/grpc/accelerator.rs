//! gRPC AcceleratorService implementation

use crate::{
    convert::{backend_health_to_proto, parse_accelerator_id, accelerator_to_proto, sample_to_proto_vec},
    error::ApiError,
    proto::{
        self,
        accelerator_service_server::AcceleratorService,
        BackendStatus, CollectMetricsRequest, CollectMetricsResponse,
        GetAcceleratorRequest, HealthCheckRequest, HealthCheckResponse,
        ListAcceleratorsRequest, ListAcceleratorsResponse, MetricSample,
        StreamMetricsRequest,
    },
};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

use tensor_guardian_domain::{
    ports::{AcceleratorBackend, BackendHealth},
    PlatformBackendFactory,
};

/// gRPC service state
#[derive(Clone)]
pub struct AcceleratorGrpcService {
    backends: Arc<Vec<Arc<dyn AcceleratorBackend>>>,
}

impl AcceleratorGrpcService {
    /// Create new service with auto-detected backends
    pub async fn new() -> anyhow::Result<Self> {
        let backends = PlatformBackendFactory::create_backends().await;
        info!("gRPC service initialized with {} backend(s)", backends.len());
        
        Ok(Self {
            backends: Arc::new(backends),
        })
    }

    /// Get backend by type
    fn get_backend(&self, backend_type: &str) -> Option<Arc<dyn AcceleratorBackend>> {
        self.backends
            .iter()
            .find(|b| b.backend_type() == backend_type)
            .cloned()
    }
}

#[tonic::async_trait]
impl AcceleratorService for AcceleratorGrpcService {
    /// List all available accelerators
    async fn list_accelerators(
        &self,
        request: Request<ListAcceleratorsRequest>,
    ) -> Result<Response<ListAcceleratorsResponse>, Status> {
        let req = request.into_inner();
        debug!("ListAccelerators request: {:?}", req.backend_filter);

        let mut accelerators = Vec::new();

        for backend in self.backends.iter() {
            let backend_type = backend.backend_type();

            // Apply backend filter if specified
            if !req.backend_filter.is_empty() {
                if !req.backend_filter.iter().any(|f| f == backend_type) {
                    continue;
                }
            }

            match backend.discover().await {
                Ok(accs) => {
                    debug!("Backend {} discovered {} accelerators", backend_type, accs.len());
                    for acc in accs {
                        accelerators.push(accelerator_to_proto(&acc));
                    }
                }
                Err(e) => {
                    warn!("Backend {} discovery failed: {}", backend_type, e);
                }
            }
        }

        info!("ListAccelerators response: {} accelerators", accelerators.len());
        Ok(Response::new(ListAcceleratorsResponse { accelerators }))
    }

    /// Get single accelerator by ID
    async fn get_accelerator(
        &self,
        request: Request<GetAcceleratorRequest>,
    ) -> Result<Response<proto::Accelerator>, Status> {
        let id = request.into_inner().id;
        debug!("GetAccelerator request: id={}", id);

        // Search across all backends
        for backend in self.backends.iter() {
            match backend.discover().await {
                Ok(accs) => {
                    if let Some(acc) = accs.iter().find(|a| a.id.to_string() == id) {
                        return Ok(Response::new(accelerator_to_proto(acc)));
                    }
                }
                Err(e) => {
                    warn!("Backend {} discovery failed: {}", backend.backend_type(), e);
                }
            }
        }

        Err(ApiError::not_found(format!("Accelerator not found: {}", id)).into())
    }

    /// Collect metrics from an accelerator
    async fn collect_metrics(
        &self,
        request: Request<CollectMetricsRequest>,
    ) -> Result<Response<CollectMetricsResponse>, Status> {
        let req = request.into_inner();
        let accel_id_str = req.accelerator_id.clone();
        debug!("CollectMetrics request: accel_id={}", accel_id_str);

        let accel_id = parse_accelerator_id(&accel_id_str)
            .map_err(|e| ApiError::invalid_request(e))?;

        // Find accelerator and its backend
        let mut target_backend = None;
        let mut target_accelerator = None;

        for backend in self.backends.iter() {
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
            .ok_or_else(|| ApiError::not_found(format!("Accelerator not found: {}", accel_id_str)))?;

        // Collect metrics
        let sample = backend
            .collect(&accelerator)
            .await
            .map_err(ApiError::from)?;

        let samples: Vec<MetricSample> = sample_to_proto_vec(&sample);
        let timestamp_ns = samples
            .first()
            .map(|s| s.timestamp_ns)
            .unwrap_or_else(|| chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

        info!(
            "CollectMetrics response: {} samples for {}",
            samples.len(),
            accel_id_str
        );

        Ok(Response::new(CollectMetricsResponse {
            accelerator_id: accel_id_str,
            timestamp_ns,
            samples,
        }))
    }

    type StreamMetricsStream = ReceiverStream<Result<MetricSample, Status>>;

    /// Stream metrics continuously
    async fn stream_metrics(
        &self,
        request: Request<StreamMetricsRequest>,
    ) -> Result<Response<Self::StreamMetricsStream>, Status> {
        let req = request.into_inner();
        let interval_ms = req.interval_ms.max(100); // Minimum 100ms
        let accel_ids: Vec<String> = req.accelerator_ids.clone();

        info!(
            "StreamMetrics request: accelerators={:?}, interval={}ms",
            accel_ids, interval_ms
        );

        // Create channel for streaming
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        let backends = self.backends.clone();

        // Spawn streaming task
        tokio::spawn(async move {
            let interval = tokio::time::Duration::from_millis(interval_ms);
            let mut ticker = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Collect metrics from all requested accelerators
                        for backend in backends.iter() {
                            match backend.discover().await {
                                Ok(accs) => {
                                    for acc in accs.iter() {
                                        // Filter by accelerator ID if specified
                                        if !accel_ids.is_empty() && !accel_ids.contains(&acc.id.to_string()) {
                                            continue;
                                        }

                                        match backend.collect(acc).await {
                                            Ok(sample) => {
                                                let samples: Vec<MetricSample> = sample_to_proto_vec(&sample);
                                                for sample in samples {
                                                    if tx.send(Ok(sample)).await.is_err() {
                                                        // Channel closed, stop streaming
                                                        return;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                warn!("Failed to collect from {}: {}", acc.id, e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    warn!("Backend {} discovery failed: {}", backend.backend_type(), e);
                                }
                            }
                        }
                    }
                }
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(stream))
    }

    /// Health check for all backends
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        debug!("HealthCheck request");

        let mut all_healthy = true;
        let mut backend_statuses = Vec::new();

        for backend in self.backends.iter() {
            let backend_type = backend.backend_type();
            let health = backend.health().await;
            let version = backend.version();

            // Count accelerators for this backend
            let accelerator_count = backend.discover().await.map(|a| a.len() as i32).unwrap_or(0);

            if !matches!(health, BackendHealth::Healthy) {
                all_healthy = false;
            }

            backend_statuses.push(backend_health_to_proto(
                backend_type,
                health,
                version,
                accelerator_count,
            ));
        }

        // If no backends, report unhealthy
        if self.backends.is_empty() {
            all_healthy = false;
            backend_statuses.push(BackendStatus {
                backend_type: "none".to_string(),
                healthy: false,
                version: "0.0.0".to_string(),
                status_message: "No backends available".to_string(),
                accelerator_count: 0,
            });
        }

        Ok(Response::new(HealthCheckResponse {
            healthy: all_healthy,
            backends: backend_statuses,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }))
    }
}
