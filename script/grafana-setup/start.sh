#!/bin/bash

# Start Grafana stack for Agent Tracer monitoring
set -e

echo "🚀 Starting Grafana stack for Agent Tracer..."

# Check if Docker is running
if ! docker info >/dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

# Start the stack
docker-compose up -d

echo "⏳ Waiting for services to start..."
sleep 10

# Check service health
echo "🔍 Checking service health..."

if curl -s http://localhost:3100/ready >/dev/null; then
    echo "✅ Loki is ready"
else
    echo "⚠️  Loki is not ready yet"
fi

if curl -s http://localhost:3000/api/health >/dev/null; then
    echo "✅ Grafana is ready"
else
    echo "⚠️  Grafana is not ready yet"
fi

echo ""
echo "🎉 Grafana stack is starting up!"
echo ""
echo "📊 Access Grafana at: http://localhost:3000"
echo "   Username: admin"
echo "   Password: admin"
echo ""
echo "🔍 Loki API at: http://localhost:3100"
echo ""
echo "📝 To view logs, make sure your Agent Tracer outputs are in:"
echo "   - ../collector/output/*.log"
echo "   - ../collector/output/*.json"
echo "   - Or any .log/.json files in the parent directory"
echo ""
echo "📈 The Agent Tracer Dashboard will be automatically available"
echo ""
echo "🛑 To stop: ./stop.sh"