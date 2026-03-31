# tensor-guardian

Universal AI accelerator monitoring with eBPF support. Multi-platform support for NVIDIA GPUs, Apple Metal/Neural Engine, Google TPUs, and custom accelerators.

[![Architecture](https://img.shields.io/badge/architecture-CKODEX-blue)](https://github.com/ckodex)
[![Rust](https://img.shields.io/badge/lang-Rust-orange)](https://www.rust-lang.org)
[![eBPF](https://img.shields.io/badge/eBPF-Aya-green)](https://aya-rs.dev)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

## Architecture

This project follows **CKODEX architectural principles**:

- **Domain-Driven Design** - Bounded contexts, aggregates, entities
- **Hexagonal Architecture** - Ports & Adapters pattern
- **Three Spaces** - Kernel/Validation/Presentation separation
- **Proof Space** - Evidence-native engineering (SBOMs, SLSA attestations)
- **Security First** - Defense in depth, zero trust

### Three Spaces Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PRESENTATION SPACE                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │     CLI     │  │     TUI     │  │   gRPC/REST API     │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
└─────────┼────────────────┼────────────────────┼─────────────┘
          │                │                    │
          ▼                ▼                    ▼
┌─────────────────────────────────────────────────────────────┐
│                    VALIDATION SPACE                          │
│  ┌─────────────────────┐  ┌──────────────────────────────┐  │
│  │   Use Cases (CQRS)  │  │     Policy Engine            │  │
│  │  - DiscoverAccels   │  │  - SamplingPolicy            │  │
│  │  - CollectMetrics   │  │  - RetentionPolicy           │  │
│  │  - AttachProbe      │  └──────────────────────────────┘  │
│  └─────────────────────┘                                    │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                      KERNEL SPACE                            │
│                    (Domain Layer)                            │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │ Aggregates  │  │  Entities   │  │   Value Objects     │  │
│  │ Accelerator │  │  Sensor     │  │  - AcceleratorId    │  │
│  │ Sample      │  │  Metric     │  │  - MetricType       │  │
│  │ MetricStream│  │  Probe      │  │  - Utilization      │  │
│  └──────┬──────┘  └─────────────┘  └─────────────────────┘  │
│         │                                                    │
│  ┌──────┴─────────────────────────────────────────────────┐   │
│  │                       PORTS                           │   │
│  │  AcceleratorBackend │ SensorBackend │ ProbeBackend    │   │
│  │  EventBus          │ Repository    │ BackendFactory  │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────┐
│                    INFRASTRUCTURE                            │
│  ┌────────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │   NVML     │  │  Metal   │  │   TPU    │  │  eBPF     │  │
│  │  Adapter   │  │ Adapter  │  │ Adapter  │  │ Adapter   │  │
│  └────────────┘  └──────────┘  └──────────┘  └───────────┘  │
└─────────────────────────────────────────────────────────────┘
```

### Proof Space (Cross-Cutting)

```
┌─────────────────────────────────────────────────────────────┐
│                      PROOF SPACE                             │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐   │
│  │   SBOM       │  │  SLSA Prov   │  │  Attestation    │   │
│  │  (Syft)      │  │  (in-toto)   │  │  (Sigstore)     │   │
│  └──────────────┘  └──────────────┘  └─────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Features

### Multi-Platform Accelerator Support

| Platform            | Status        | Backend    |
| ------------------- | ------------- | ---------- |
| NVIDIA GPU (Linux)  | ✅ Implemented | NVML       |
| Apple Metal/ANE     | 🚧 Planned     | IOKit      |
| Google Cloud TPU    | 🚧 Planned     | TPU Driver |
| AMD ROCm            | 🚧 Planned     | ROCm SMI   |
| Intel NPU           | 🚧 Planned     | Level Zero |
| Custom/User-defined | 🚧 Planned     | Plugin API |

### eBPF Kernel Instrumentation

- GPU memory allocation tracking
- DMA/PCIe I/O tracing
- Memory pressure detection
- Scheduler events

### Output Modes

- **TUI** - Interactive terminal interface (ratatui)
- **Headless** - CSV logging, Prometheus exporter
- **gRPC API** - For integration with observability platforms

## Building

### Prerequisites

- Rust 1.75+
- Linux kernel 5.8+ (for eBPF)
- LLVM/Clang (for eBPF compilation)
- NVIDIA drivers + NVML (for NVIDIA GPU support)

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get install -y \
    llvm clang libelf-dev \
    libssl-dev pkg-config protobuf-compiler

# Install cargo-bpf for eBPF
cargo install cargo-bpf
```

### Build Commands

```bash
# Standard build
cargo build --release

# With NVML support
cargo build --release --features nvml

# With eBPF support
cargo build --release --features ebpf

# All features
cargo build --release --features "nvml,ebpf,tui"
```

### Dagger CI/CD (SLSA-compliant)

```bash
# Run full pipeline
cd .dagger && go run src/main.go release

# Individual steps
go run src/main.go build --target x86_64-unknown-linux-gnu
go run src/main.go test
go run src/main.go sbom
go run src/main.go attest
```

## Usage

### CLI

```bash
# Discover accelerators
tensor-guardian discover

# Interactive TUI mode
tensor-guardian monitor

# Headless mode with CSV logging
tensor-guardian monitor -n -l stats.csv -i 5000

# Prometheus exporter
tensor-guardian monitor -p 9101

# Check available backends
tensor-guardian backends
```

### Configuration

Create `~/.config/tensor-guardian/config.toml`:

```toml
[sampling]
interval_ms = 1000
retention = "LastN(1000)"

[backends.nvml]
enabled = true

[backends.metal]
enabled = false

[export.prometheus]
port = 9101
host = "0.0.0.0"
path = "/metrics"

[export.csv]
path = "/var/log/tensor-guardian/metrics.csv"
headers = true
flush_interval_secs = 60

[tui]
color_scheme = "default"
show_history = true
history_size = 20
```

## Project Structure

```
tensor-guardian/
├── kernel/
│   ├── domain/              # Pure domain logic
│   │   ├── aggregates/      # Accelerator, Sample, MetricStream
│   │   ├── entities/        # Sensor, Metric, Probe
│   │   ├── events/          # Domain events
│   │   ├── ports/           # Backend interfaces
│   │   └── value_objects/   # IDs, Types, Units
│   └── adapters/
│       ├── nvml/            # NVIDIA NVML backend
│       ├── metal/           # Apple Metal (TODO)
│       ├── tpu/             # Google TPU (TODO)
│       ├── ebpf/            # eBPF kernel probes
│       └── procfs/          # Linux /proc parsing
├── validation/
│   ├── app/                 # Use cases (CQRS)
│   └── policies/            # Business rules
├── presentation/
│   ├── cli/                 # Command-line interface
│   ├── tui/                 # Terminal UI
│   └── api/                 # gRPC/REST API (TODO)
├── proof/
│   ├── attestation/         # SLSA/in-toto
│   └── telemetry/           # OpenTelemetry (TODO)
├── ebpf-programs/           # Kernel eBPF programs
│   ├── gpu_events/
│   ├── io_trace/
│   └── memory_pressure/
├── .dagger/                 # CI/CD pipeline
└── proto/                   # gRPC contracts (Buf v2)
```

## Domain Language

| Term            | Definition                                             |
| --------------- | ------------------------------------------------------ |
| **Accelerator** | Hardware device for AI/ML compute (GPU, TPU, NPU)      |
| **Sensor**      | Data source producing metrics (hardware, kernel probe) |
| **Metric**      | Measurable value with timestamp and labels             |
| **Sample**      | Point-in-time collection of metrics                    |
| **Stream**      | Continuous flow of samples                             |
| **Backend**     | Platform-specific adapter for accelerator type         |
| **Probe**       | eBPF kernel instrumentation point                      |

## Security & Evidence

### SLSA Compliance

- **Level 3+** - Build service generates provenance
- **SBOMs** - CycloneDX and SPDX formats
- **Attestations** - in-toto/SLSA provenance with Sigstore signing

### Supply Chain Security

- Dependency scanning with `cargo audit`
- Reproducible builds via Dagger
- Signed releases with cosign

## License

MIT License - See [LICENSE](LICENSE)

## Acknowledgments

- Built with [aya-rs](https://aya-rs.dev) for eBPF
- TUI powered by [ratatui](https://ratatui.rs)
- NVIDIA monitoring via [nvml-wrapper](https://github.com/Cldfire/nvml-wrapper)
- Architecture following CKODEX principles


Accurately monitor a single machine or an entire cluster with minimal overhead. Reports metrics to NVIDIA specifications via NVML, with correct handling of unified memory, HugePages, and ARM big.LITTLE core topology. Includes `demo-load`, a zero-dependency synthetic CPU/GPU load generator for validating your monitoring pipeline end-to-end.

![C](https://img.shields.io/badge/lang-C-blue) ![License](https://img.shields.io/badge/license-MIT-green) ![Arch](https://img.shields.io/badge/arch-aarch64%20%7C%20x86__64-orange)

## Display

### CPU Section
- **Overall** aggregate usage bar across all cores
- **Per-core** usage bars in dual-column layout with ARM core type labels (**X925** = performance cores at 3.9 GHz, **X725** = efficiency cores at 2.8 GHz on the Grace big.LITTLE architecture)
- CPU temperature (highest thermal zone) and frequency

### Memory Section
- **Used** (green) — actual application memory (total - free - buffers - cached)
- **Buf/cache** (blue) — kernel buffers and page cache (reclaimable)
- Swap usage bar
- Correctly handles **HugePages** on DGX Spark where `MemAvailable` is inaccurate

### GPU Section
- **GPU utilization** bar with temperature, power draw (watts), and clock speed
- **VRAM** bar, or "unified memory" label on DGX Spark where CPU/GPU share memory
- **ENC/DEC** — hardware video encoder (NVENC) and decoder (NVDEC) utilization percentage

### GPU Processes
- **PID** — process ID
- **USER** — process owner
- **TYPE** — **C** (Compute: CUDA/inference workloads) or **G** (Graphics: rendering, e.g. Xorg)
- **CPU%** — per-process CPU usage (delta-based, per-core scale)
- **GPU MEM** — GPU memory allocated by the process
- **COMMAND** — binary name with arguments
- **(other processes)** — summary row showing CPU usage from non-GPU processes

### History Chart
- Full-width rolling graph of CPU (green) and GPU (cyan) utilization over the last 20 samples using Unicode block elements (▁▂▃▄▅▆▇█)

### General
- Color-coded bars: green (normal), yellow (>60%), red (>90%)
- **CSV Logging** — log all stats to file with configurable interval
- **Headless Mode** — run without TUI for unattended data collection
- 1s default refresh, adjustable at runtime or via CLI
- NVML loaded dynamically at runtime — no hard dependency on NVIDIA drivers

<table>
<tr>
<td><strong>aarch64</strong> (DGX Spark — Grace + GB10)</td>
<td><strong>x86_64</strong> (Laptop — Ryzen 7 + RTX 3050)</td>
</tr>
<tr>
<td><img src="nv-monitor-arm.png" alt="nv-monitor on ARM"></td>
<td><img src="nv-monitor-x86.png" alt="nv-monitor on x86"></td>
</tr>
</table>

## Download

For the reckless among you, there's a [binary release](https://github.com/wentbackward/nv-monitor/releases) you can download if you don't want to build it yourself.

## Building

Requires `gcc` and `libncurses-dev`:

```bash
sudo apt install build-essential libncurses-dev
make
```

## Usage

```bash
./nv-monitor                           # TUI only
./nv-monitor -l stats.csv              # TUI + log every 1s
./nv-monitor -l stats.csv -i 5000      # TUI + log every 5s
./nv-monitor -n -l stats.csv -i 500    # Headless, log every 500ms
./nv-monitor -r 2000                   # TUI refreshing every 2s
./nv-monitor -p 9101                   # TUI + Prometheus metrics on :9101
./nv-monitor -n -p 9101                # Headless Prometheus exporter
```

Or install system-wide:

```bash
sudo make install
```

### Command-line options

| Flag      | Description                                | Default |
| --------- | ------------------------------------------ | ------- |
| `-l FILE` | Log statistics to CSV file                 | off     |
| `-i MS`   | Log interval in milliseconds               | 1000    |
| `-n`      | Headless mode (no TUI, requires `-l`/`-p`) | off     |
| `-p PORT` | Expose Prometheus metrics on PORT          | off     |
| `-r MS`   | UI refresh interval in milliseconds        | 1000    |
| `-v`      | Show version                               |         |
| `-h`      | Show help                                  |         |

### Interactive controls

| Key     | Action                            |
| ------- | --------------------------------- |
| `q`/Esc | Quit                              |
| `s`     | Toggle sort (GPU memory / PID)    |
| `+`/`-` | Adjust refresh rate (250ms steps) |

## Prometheus Metrics

Pass `-p PORT` to expose a Prometheus-compatible metrics endpoint:

```bash
./nv-monitor -p 9101              # TUI + metrics at http://localhost:9101/metrics
./nv-monitor -n -p 9101           # Pure headless exporter
curl -s localhost:9101/metrics     # Check it works
```

### Available metrics

| Metric                               | Type  | Labels        | Description                                                  |
| ------------------------------------ | ----- | ------------- | ------------------------------------------------------------ |
| `nv_build_info`                      | gauge | `version`     | nv-monitor version                                           |
| `nv_uptime_seconds`                  | gauge |               | System uptime                                                |
| `nv_load_average`                    | gauge | `interval`    | Load average (1m, 5m, 15m)                                   |
| `nv_cpu_usage_percent`               | gauge | `cpu`, `type` | Per-core CPU utilization (type = ARM core: X925, X725, etc.) |
| `nv_cpu_temperature_celsius`         | gauge |               | CPU temperature                                              |
| `nv_cpu_frequency_mhz`               | gauge |               | CPU frequency                                                |
| `nv_memory_total_bytes`              | gauge |               | Total system memory                                          |
| `nv_memory_used_bytes`               | gauge |               | Application memory used                                      |
| `nv_memory_bufcache_bytes`           | gauge |               | Buffer and cache memory                                      |
| `nv_swap_total_bytes`                | gauge |               | Total swap                                                   |
| `nv_swap_used_bytes`                 | gauge |               | Swap used                                                    |
| `nv_gpu_info`                        | gauge | `gpu`, `name` | GPU device name                                              |
| `nv_gpu_utilization_percent`         | gauge | `gpu`         | GPU compute utilization                                      |
| `nv_gpu_temperature_celsius`         | gauge | `gpu`         | GPU temperature                                              |
| `nv_gpu_power_watts`                 | gauge | `gpu`         | GPU power draw                                               |
| `nv_gpu_clock_mhz`                   | gauge | `gpu`, `type` | GPU clock speed (graphics, memory)                           |
| `nv_gpu_memory_total_bytes`          | gauge | `gpu`         | GPU memory total                                             |
| `nv_gpu_memory_used_bytes`           | gauge | `gpu`         | GPU memory used                                              |
| `nv_gpu_fan_speed_percent`           | gauge | `gpu`         | Fan speed                                                    |
| `nv_gpu_encoder_utilization_percent` | gauge | `gpu`         | Hardware encoder utilization                                 |
| `nv_gpu_decoder_utilization_percent` | gauge | `gpu`         | Hardware decoder utilization                                 |

### Prometheus scrape config

```yaml
scrape_configs:
  - job_name: 'nv-monitor'
    static_configs:
      - targets: ['dgx-spark:9101']
```

No new dependencies are required — the exporter uses POSIX sockets and adds ~128 KB of memory overhead.

## Synthetic Load Testing

A companion tool `demo-load` generates sinusoidal CPU and GPU loads for visual testing and multi-node validation — no bulky benchmarking tools required. See [DEMO-LOAD.md](DEMO-LOAD.md) for details.

```bash
make demo-load
./demo-load --gpu          # CPU + GPU sinusoidal load
```

## Requirements

- Linux (reads from `/proc` and `/sys`)
- ncurses (TUI mode)
- NVIDIA drivers with NVML (for GPU monitoring — CPU/memory work without it)

### Platform support

| Platform                          | Status                                                                                    |
| --------------------------------- | ----------------------------------------------------------------------------------------- |
| DGX Spark (aarch64, Grace + GB10) | Primary target — full support including unified memory, HugePages, big.LITTLE core labels |
| Any Linux + NVIDIA GPU (x86_64)   | Fully supported — CPU, memory, GPU, processes, Prometheus exporter                        |
| Linux without NVIDIA GPU          | CPU and memory monitoring only, GPU section shows "NVML not available"                    |

## License

MIT
