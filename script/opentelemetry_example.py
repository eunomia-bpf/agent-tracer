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

SSL_TIMELINE_FILE = '/home/yunwei37/agent-tracer/script/results/ssl_only/claude_code/analysis_gen_readme/ssl_timeline.json'

try:
    with open(SSL_TIMELINE_FILE, 'r') as f:
        ssl_data = json.load(f)
except FileNotFoundError:
    logging.error(f"Error: {SSL_TIMELINE_FILE} not found.")
    exit(1)
except json.JSONDecodeError:
    logging.error(f"Error: Could not decode JSON from {SSL_TIMELINE_FILE}.")
    exit(1)

with tracer.start_as_current_span("process_ssl_timeline") as parent_span:
    parent_span.set_attribute("timeline.source_file", ssl_data['analysis_metadata']['source_file'])
    parent_span.set_attribute("timeline.total_entries", ssl_data['analysis_metadata']['total_timeline_entries'])

    logging.info(f"Processing SSL timeline from {SSL_TIMELINE_FILE}")

    for entry in ssl_data['timeline']:
        with tracer.start_as_current_span(f"timeline_entry_{entry.get('type', 'unknown')}") as child_span:
            child_span.set_attribute("entry.type", entry.get('type'))
            child_span.set_attribute("entry.function", entry.get('function'))
            child_span.set_attribute("entry.pid", entry.get('pid'))
            child_span.set_attribute("entry.comm", entry.get('comm'))

            # Add relevant data from the entry as log attributes
            log_attributes = {
                "entry_type": entry.get('type'),
                "entry_function": entry.get('function'),
                "entry_pid": entry.get('pid'),
                "entry_comm": entry.get('comm'),
                "entry_timestamp": entry.get('timestamp'),
            }

            if entry.get('type') == 'request':
                log_attributes["request_method"] = entry.get('method')
                log_attributes["request_path"] = entry.get('path')
                log_attributes["request_host"] = entry.get('headers', {}).get('host')
                logging.info(f"Request: {entry.get('method')} {entry.get('path')}", extra=log_attributes)
            elif entry.get('type') == 'response':
                log_attributes["response_status_code"] = entry.get('status_code')
                log_attributes["response_status_text"] = entry.get('status_text')
                logging.info(f"Response: {entry.get('status_code')} {entry.get('status_text')}", extra=log_attributes)
            else:
                logging.info(f"Other entry type: {entry.get('type')}", extra=log_attributes)

print("\nSSL timeline processing complete. Spans and logs have been sent to Jaeger.")
print("You can view the traces and logs in the Jaeger UI.")

# The script will exit here, but the exporter will send the spans and logs in the background.
