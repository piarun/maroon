use libp2p::PeerId;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry_otlp::MetricExporter;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::metrics::Temporality;

pub fn init_meter_provider(id: PeerId) -> Result<SdkMeterProvider, Box<dyn std::error::Error>> {
  let endpoint = std::env::var("OTEL_EXPORTER_OTLP_GRPC_ENDPOINT")
    .map_err(|e| format!("OTEL_EXPORTER_OTLP_GRPC_ENDPOINT not set: {}", e))?;

  let exporter = MetricExporter::builder()
    .with_tonic()
    .with_endpoint(endpoint)
    .with_temporality(Temporality::Cumulative)
    .build()
    .expect("exp");

  let resource =
    Resource::builder_empty().with_attribute(KeyValue::new("id", id.to_string())).with_service_name("maroon").build();

  let provider = SdkMeterProvider::builder().with_periodic_exporter(exporter).with_resource(resource).build();
  global::set_meter_provider(provider.clone());
  Ok(provider)
}
