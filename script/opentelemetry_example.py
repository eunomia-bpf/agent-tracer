import time
import json
import logging
from opentelemetry import trace
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import SimpleSpanProcessor
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk._logs import LoggerProvider, LoggingHandler
from opentelemetry.sdk._logs.export import ConsoleLogExporter, SimpleLogRecordProcessor
from opentelemetry.exporter.otlp.proto.grpc._log_exporter import OTLPLogExporter

# --- 1. Set up OpenTelemetry Tracing ---

# Create a Resource to identify our service
resource = Resource(attributes={
    "service.name": "ssl-timeline-analyzer"
})

# A TracerProvider is the entry point of the API.
# It provides access to Tracers.
trace.set_tracer_provider(TracerProvider(resource=resource))

# A Tracer is an object that can be used to create spans.
tracer = trace.get_tracer(__name__)

# --- 2. Configure OpenTelemetry Logging ---

# Create a LoggerProvider
logger_provider = LoggerProvider(resource=resource)

# Configure OTLPLogExporter to send logs to Jaeger/OTLP collector
otlp_log_exporter = OTLPLogExporter()
logger_provider.add_log_record_processor(SimpleLogRecordProcessor(otlp_log_exporter))

# Attach the OpenTelemetry handler to the root logger
handler = LoggingHandler(level=logging.INFO, logger_provider=logger_provider)
logging.getLogger().addHandler(handler)
logging.getLogger().setLevel(logging.INFO)

# --- 3. Configure Tracing Exporter ---

# Create an OTLP Span Exporter to send spans to Jaeger.
# The default endpoint is localhost:4317, which is where our Jaeger container is listening.
otlp_span_exporter = OTLPSpanExporter()

# A SpanProcessor is responsible for processing the spans before they are exported.
span_processor = SimpleSpanProcessor(otlp_span_exporter)

# Add the span processor to the tracer provider.
trace.get_tracer_provider().add_span_processor(span_processor)

# --- 4. Read and Process SSL Timeline Data ---

SSL_TIMELINE_FILE = '/home/yunwei37/agent-tracer/script/results/ssl_only/claude_code/analysis_modify_code/ssl_data_only.json'

try:
    with open(SSL_TIMELINE_FILE, 'r') as f:
        ssl_data = json.load(f)
except FileNotFoundError:
    logging.error(f"Error: {SSL_TIMELINE_FILE} not found.")
    exit(1)
except json.JSONDecodeError:
    logging.error(f"Error: Could not decode JSON from {SSL_TIMELINE_FILE}.")
    exit(1)

with tracer.start_as_current_span("process_ssl_data") as parent_span:
    parent_span.set_attribute("data.source_file", ssl_data['analysis_metadata']['source_file'])
    parent_span.set_attribute("data.total_entries", ssl_data['summary']['total_data_entries'])
    parent_span.set_attribute("data.total_requests", ssl_data['summary']['total_requests'])
    parent_span.set_attribute("data.total_responses", ssl_data['summary']['total_responses'])
    parent_span.set_attribute("data.total_bytes", ssl_data['summary']['data_transfer_sizes']['total_data_bytes'])
    parent_span.set_attribute("session.duration_seconds", ssl_data['summary']['session_duration_seconds'])

    logging.info(f"Processing SSL data from {SSL_TIMELINE_FILE}")
    logging.info(f"Total entries: {ssl_data['summary']['total_data_entries']}, Requests: {ssl_data['summary']['total_requests']}, Responses: {ssl_data['summary']['total_responses']}")

    for entry in ssl_data['data_timeline']:
        with tracer.start_as_current_span(f"ssl_data_{entry.get('type', 'unknown')}") as child_span:
            child_span.set_attribute("entry.type", entry.get('type'))
            child_span.set_attribute("entry.timestamp", entry.get('timestamp'))
            child_span.set_attribute("entry.tid", entry.get('tid'))

            # Add span events for HTTP requests and responses
            if entry.get('type') == 'request':
                child_span.set_attribute("http.method", entry.get('method'))
                child_span.set_attribute("http.path", entry.get('path'))
                
                # Extract JSON body data for analysis
                json_body = entry.get('json_body', {})
                if json_body:
                    events = json_body.get('events', [])
                    if events:
                        event = events[0]
                        child_span.set_attribute("event.name", event.get('eventName'))
                        metadata = event.get('metadata', {})
                        child_span.set_attribute("event.model", metadata.get('model'))
                        child_span.set_attribute("event.provider", metadata.get('provider'))
                        child_span.set_attribute("event.session_id", metadata.get('sessionId'))
                        child_span.set_attribute("event.user_type", metadata.get('userType'))
                
                child_span.add_event("HTTP Request", {
                    "http.method": entry.get('method'),
                    "http.path": entry.get('path'),
                    "body_size": len(entry.get('body', '')),
                    "has_json_body": bool(entry.get('json_body'))
                })
                
            elif entry.get('type') == 'response':
                # For responses, extract any available metadata
                json_body = entry.get('json_body', {})
                if json_body and isinstance(json_body, dict):
                    child_span.set_attribute("response.has_data", True)
                    if 'error' in json_body:
                        child_span.set_attribute("response.has_error", True)
                    if 'completion' in json_body:
                        child_span.set_attribute("response.has_completion", True)
                
                child_span.add_event("HTTP Response", {
                    "body_size": len(entry.get('body', '')),
                    "has_json_body": bool(entry.get('json_body'))
                })

print("\nSSL timeline processing complete. Spans and logs have been sent to Jaeger.")
print("You can view the traces and logs in the Jaeger UI.")

# The script will exit here, but the exporter will send the spans and logs in the background.
