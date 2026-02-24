//! OpenTelemetry OTLP pipeline initialization.
//!
//! Call [`setup`] at process startup to install a global tracing subscriber that
//! exports spans via OTLP/HTTP to the configured collector (Jaeger, Grafana, etc.).
//!
//! The [`OtelGuard`] returned from [`setup`] must be kept alive for the duration of
//! the process — dropping it flushes pending spans and shuts down the pipeline.
//!
//! # Example
//! ```no_run
//! # use vibecli::otel_init;
//! # use vibecli::config::OtelConfig;
//! let guard = otel_init::setup(&OtelConfig::default()).unwrap();
//! // ... run agent ...
//! drop(guard); // flushes remaining spans
//! ```

use crate::config::OtelConfig;
use anyhow::Result;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::filter::EnvFilter;

/// Guard that shuts down the OTel pipeline when dropped.
pub struct OtelGuard {
    provider: SdkTracerProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(e) = self.provider.shutdown() {
            eprintln!("[otel] Shutdown error: {e}");
        }
    }
}

/// Initialize the global tracing subscriber with an OTLP exporter.
///
/// If OTel is disabled in config, a simple `fmt` subscriber is installed instead
/// (respecting `RUST_LOG`). Returns `None` if OTel is disabled.
///
/// Call this once, early in `main()`.
pub fn setup(config: &OtelConfig) -> Result<Option<OtelGuard>> {
    if !config.enabled {
        // Install a simple stderr subscriber when OTLP is not requested.
        // (Won't conflict if one is already set.)
        let _ = tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(tracing_subscriber::fmt::layer())
            .try_init();
        return Ok(None);
    }

    // ── Build OTLP/HTTP exporter ───────────────────────────────────────────────
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_endpoint(config.endpoint.clone())
        .build()?;

    // ── Build SDK tracer provider ──────────────────────────────────────────────
    let provider = SdkTracerProvider::builder()
        .with_resource(opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME,
                config.service_name.clone(),
            ),
        ]))
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .build();

    // ── Bridge: tracing → opentelemetry ───────────────────────────────────────
    let tracer = provider.tracer(config.service_name.clone());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .try_init()
        .map_err(|e| anyhow::anyhow!("Failed to initialize tracing subscriber: {e}"))?;

    eprintln!(
        "[otel] Exporting spans to {} (service: {})",
        config.endpoint, config.service_name
    );

    Ok(Some(OtelGuard { provider }))
}
