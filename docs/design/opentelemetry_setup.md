# OpenTelemetry and Jaeger Setup for Trace Visualization

This document outlines the steps to set up Jaeger and run the provided OpenTelemetry Python example (`opentelemetry_example.py`) to visualize traces.

## Prerequisites

*   Docker (for running Jaeger)
*   Python 3.x
*   `opentelemetry-sdk` and `opentelemetry-exporter-otlp` Python packages. You can install them using pip:
    ```bash
    pip install opentelemetry-sdk opentelemetry-exporter-otlp
    ```

## 1. Run Jaeger with Docker

Jaeger provides an all-in-one Docker image that includes the collector, query, and UI.

To start Jaeger, run the following command in your terminal:

```bash
docker run --replace -d --name jaeger -p 16686:16686 -p 4317:4317 -p 4318:4318 jaegertracing/all-in-one:latest
```

*   `-d`: Runs the container in detached mode (in the background).
*   `--name jaeger`: Assigns a name to the container for easy reference.
*   `-p 16686:16686`: Maps port 16686 from your host to the container, which is where the Jaeger UI is accessible.
*   `-p 4317:4317`: Maps port 4317 (gRPC) for OTLP (OpenTelemetry Protocol) trace ingestion.
*   `-p 4318:4318`: Maps port 4318 (HTTP) for OTLP trace ingestion.
*   `jaegertracing/all-in-one:latest`: Specifies the Docker image to use.

## 2. Understand the `opentelemetry_example.py` Script

The `opentelemetry_example.py` script demonstrates how to instrument a simple Python application with OpenTelemetry to send traces to a Jaeger collector.

Key components:

*   **Resource**: Identifies your service (e.g., `service.name: "my-local-demo-app"`).
*   **TracerProvider**: The entry point for OpenTelemetry, providing `Tracer` instances.
*   **OTLPSpanExporter**: Configured to send traces to the Jaeger collector, which by default listens on `localhost:4317` (gRPC).
*   **SimpleSpanProcessor**: Processes spans and sends them to the exporter.
*   **Spans**: Created using `tracer.start_as_current_span()`, representing units of work. Spans can have attributes and events.

## 3. Run the Python Example

Navigate to the `script` directory in your terminal:

```bash
cd /home/yunwei37/agent-tracer/script
```

Then, execute the Python script:

```bash
python opentelemetry_example.py
```

This script will generate traces and send them to the Jaeger collector running in your Docker container.

## 4. View Traces in Jaeger UI

After running the Python script, open your web browser and go to the Jaeger UI:

[http://localhost:16686](http://localhost:16686)

In the Jaeger UI:

1.  Select `my-local-demo-app` from the "Service" dropdown.
2.  Click the "Find Traces" button.

You should see the `main_operation` trace, which includes the `sub_task` span, along with their attributes and events.
