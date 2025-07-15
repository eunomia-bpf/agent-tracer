#!/bin/bash

# View logs from Grafana stack components
set -e

case "${1:-all}" in
    "grafana")
        echo "📊 Grafana logs:"
        docker-compose logs -f grafana
        ;;
    "loki")
        echo "🔍 Loki logs:"
        docker-compose logs -f loki
        ;;
    "promtail")
        echo "📝 Promtail logs:"
        docker-compose logs -f promtail
        ;;
    "all"|*)
        echo "📋 All service logs:"
        docker-compose logs -f
        ;;
esac