# Lightweight Grafana Setup for Agent Tracer

This setup provides a minimal Grafana stack optimized for visualizing JSON logs from the Agent Tracer project.

## Stack Components

- **Grafana** (10.1.0): Web UI for visualization
- **Loki** (2.9.0): Log aggregation system
- **Promtail** (2.9.0): Log collection agent

## Quick Start

```bash
# Start the stack
./start.sh

# Access Grafana at http://localhost:3000
# Username: admin, Password: admin

# Stop the stack
./stop.sh

# View logs
./logs.sh [grafana|loki|promtail]
```

## Log Sources

Promtail is configured to monitor:
- `../*.log` - General log files in parent directory
- `../*.json` - JSON log files in parent directory
- `/logs/ssl_*.log` - SSL/TLS monitoring logs
- `/logs/process_*.log` - Process monitoring logs

## Dashboard Features

The pre-configured "Agent Tracer Dashboard" includes:
- Event types timeline
- Top processes by event count
- Live log stream
- SSL data types distribution
- Active process count

## Configuration

### Adding New Log Sources

Edit `promtail-config.yml` to add new scrape configs:

```yaml
scrape_configs:
  - job_name: my-custom-logs
    static_configs:
      - targets: [localhost]
        labels:
          job: my-custom-logs
          __path__: /logs/custom_*.log
```

### Customizing Retention

Edit `loki-config.yml`:

```yaml
limits_config:
  retention_period: 168h  # 7 days (default: 31 days)
```

## Resource Usage

This setup is optimized for minimal resource usage:
- Loki uses filesystem storage (no external dependencies)
- Limited retention (31 days)
- Reduced query limits for memory efficiency
- Single-instance deployment

## Troubleshooting

### No Data in Grafana
1. Check if log files exist in monitored paths
2. Verify Promtail is reading files: `./logs.sh promtail`
3. Check Loki ingestion: `curl http://localhost:3100/metrics`

### High Memory Usage
1. Reduce retention period in `loki-config.yml`
2. Adjust `max_query_series` in limits_config
3. Monitor with: `docker stats`

### Log Parsing Issues
1. Verify JSON format matches pipeline stages in `promtail-config.yml`
2. Check timestamp format compatibility
3. View Promtail logs for parsing errors

## Data Persistence

- Grafana data: `grafana-data` volume
- Loki data: `loki-data` volume
- To reset: `docker-compose down -v`