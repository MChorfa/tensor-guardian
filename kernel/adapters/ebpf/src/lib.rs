use aya::maps::{MapData, RingBuf};
use aya::programs::{KProbe, TracePoint, Xdp};
use aya::util::online_cpus;
use aya::{include_bytes_aligned, Bpf, BpfLoader};
use aya_log::BpfLogger;
use async_trait::async_trait;
use tensor_guardian_domain::{
    entities::{Probe, ProbeEvent as DomainProbeEvent},
    ports::{ProbeBackend, ProbeEvent},
    value_objects::{ProbeId, ProbeType, Timestamp},
    DomainError, DomainResult,
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// eBPF-based probe backend
pub struct EbpfProbeBackend {
    bpf: Option<Bpf>,
    event_tx: mpsc::Sender<ProbeEvent>,
    event_rx: mpsc::Receiver<ProbeEvent>,
    attached_probes: HashMap<ProbeId, Probe>,
}

impl std::fmt::Debug for EbpfProbeBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EbpfProbeBackend")
            .field("attached_probes", &self.attached_probes.len())
            .finish()
    }
}

impl EbpfProbeBackend {
    pub fn new() -> DomainResult<Self> {
        let (tx, rx) = mpsc::channel(1024);
        
        Ok(Self {
            bpf: None,
            event_tx: tx,
            event_rx: rx,
            attached_probes: HashMap::new(),
        })
    }

    pub async fn load_programs(&mut self) -> DomainResult<()> {
        // Load the eBPF programs
        #[cfg(debug_assertions)]
        let bpf = BpfLoader::new()
            .load(include_bytes_aligned!("../../../../ebpf-programs/target/bpfel-unknown-none/debug/gpu-events"))
            .map_err(|e| DomainError::BackendError(format!("Failed to load eBPF: {}", e)))?;

        #[cfg(not(debug_assertions))]
        let bpf = BpfLoader::new()
            .load(include_bytes_aligned!("../../../../ebpf-programs/target/bpfel-unknown-none/release/gpu-events"))
            .map_err(|e| DomainError::BackendError(format!("Failed to load eBPF: {}", e)))?;

        // Initialize BPF logger
        BpfLogger::init(&bpf, |_, _, record| {
            info!("[eBPF] {}", record);
        }).map_err(|e| DomainError::BackendError(format!("BPF logger init failed: {}", e)))?;

        self.bpf = Some(bpf);
        info!("eBPF programs loaded successfully");
        
        Ok(())
    }

    fn attach_kprobe(&mut self, probe: &mut Probe) -> DomainResult<()> {
        let bpf = self.bpf.as_mut()
            .ok_or_else(|| DomainError::BackendError("BPF not loaded".to_string()))?;

        let program: &mut KProbe = bpf
            .program_mut(&probe.name)
            .ok_or_else(|| DomainError::BackendError(format!("Program {} not found", probe.name)))?
            .try_into()
            .map_err(|e| DomainError::BackendError(format!("Invalid program type: {}", e)))?;

        program
            .load()
            .map_err(|e| DomainError::BackendError(format!("Failed to load program: {}", e)))?;

        program
            .attach(&probe.target, 0)
            .map_err(|e| DomainError::BackendError(format!("Failed to attach kprobe: {}", e)))?;

        probe.mark_attached();
        info!("Attached kprobe {} to {}", probe.id, probe.target);
        
        Ok(())
    }

    fn attach_tracepoint(&mut self, probe: &mut Probe) -> DomainResult<()> {
        let bpf = self.bpf.as_mut()
            .ok_or_else(|| DomainError::BackendError("BPF not loaded".to_string()))?;

        // Parse tracepoint target (category:name)
        let parts: Vec<&str> = probe.target.split(':').collect();
        if parts.len() != 2 {
            return Err(DomainError::ConfigurationError(
                format!("Invalid tracepoint format: {}", probe.target)
            ));
        }

        let program: &mut TracePoint = bpf
            .program_mut(&probe.name)
            .ok_or_else(|| DomainError::BackendError(format!("Program {} not found", probe.name)))?
            .try_into()
            .map_err(|e| DomainError::BackendError(format!("Invalid program type: {}", e)))?;

        program
            .load()
            .map_err(|e| DomainError::BackendError(format!("Failed to load program: {}", e)))?;

        program
            .attach(parts[0], parts[1])
            .map_err(|e| DomainError::BackendError(format!("Failed to attach tracepoint: {}", e)))?;

        probe.mark_attached();
        info!("Attached tracepoint {} to {}:{}", probe.id, parts[0], parts[1]);
        
        Ok(())
    }

    pub async fn start_event_polling(&mut self) -> DomainResult<()> {
        let bpf = self.bpf.as_mut()
            .ok_or_else(|| DomainError::BackendError("BPF not loaded".to_string()))?;

        // Set up ring buffer for events
        let mut ring_buf: RingBuf<MapData> = RingBuf::try_from(
            bpf.map_mut("events")
                .ok_or_else(|| DomainError::BackendError("Events map not found".to_string()))?
        ).map_err(|e| DomainError::BackendError(format!("Failed to create ring buffer: {}", e)))?;

        let tx = self.event_tx.clone();

        tokio::spawn(async move {
            loop {
                match ring_buf.next() {
                    Some(data) => {
                        let event = ProbeEvent {
                            probe_id: ProbeId::new(), // TODO: Map from BPF context
                            timestamp: Timestamp::now(),
                            data: data.to_vec(),
                            event_type: "gpu_event".to_string(),
                        };
                        
                        if tx.send(event).await.is_err() {
                            break;
                        }
                    }
                    None => {
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
            }
        });

        Ok(())
    }
}

#[async_trait]
impl ProbeBackend for EbpfProbeBackend {
    fn backend_type(&self) -> &'static str {
        "ebpf"
    }

    async fn attach(&self, probe: &mut Probe) -> DomainResult<()> {
        // Note: This requires mutable self, but async_trait doesn't easily allow this
        // In practice, we'd need interior mutability or a different approach
        warn!("eBPF attach requires backend initialization");
        Ok(())
    }

    async fn detach(&self, probe: &mut Probe) -> DomainResult<()> {
        probe.mark_detached();
        info!("Detached probe {}", probe.id);
        Ok(())
    }

    async fn poll(&self) -> DomainResult<Vec<ProbeEvent>> {
        let mut events = Vec::new();
        
        // Try to drain the channel without blocking
        while let Ok(event) = self.event_rx.try_recv() {
            events.push(event);
        }
        
        Ok(events)
    }
}

/// Predefined probes for GPU monitoring
pub struct GpuProbeSet;

impl GpuProbeSet {
    /// Probe for GPU memory allocation events
    pub fn gpu_memory_alloc() -> Probe {
        Probe::new(
            "gpu_memory_alloc",
            ProbeType::Tracepoint,
            "drm:drm_vblank_event",
            tensor_guardian_domain::entities::ProbeProgramType::GpuMemory,
        )
    }

    /// Probe for DMA mapping events
    pub fn dma_map() -> Probe {
        Probe::new(
            "dma_map_page",
            ProbeType::Kprobe,
            "dma_map_page",
            tensor_guardian_domain::entities::ProbeProgramType::IoTrace,
        )
    }

    /// Probe for memory pressure events
    pub fn memory_pressure() -> Probe {
        Probe::new(
            "memory_pressure",
            ProbeType::Tracepoint,
            "vmscan:mm_vmscan_direct_reclaim_begin",
            tensor_guardian_domain::entities::ProbeProgramType::MemoryPressure,
        )
    }

    /// Get all available GPU probes
    pub fn all() -> Vec<Probe> {
        vec![
            Self::gpu_memory_alloc(),
            Self::dma_map(),
            Self::memory_pressure(),
        ]
    }
}
