#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker-compose.yml"
ENV_FILE=".env"

echo -e "${GREEN}🚀 Starting deployment of Image Hosting Server${NC}"

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}❌ Docker is not running. Please start Docker first.${NC}"
    exit 1
fi

# Check if .env file exists
if [ ! -f "$ENV_FILE" ]; then
    echo -e "${YELLOW}⚠️  .env file not found. Creating from template...${NC}"
    cp .env.example .env
    echo -e "${YELLOW}📝 Please edit .env file with your configuration before continuing.${NC}"
    exit 1
fi

# Load environment variables
source .env

# Validate required environment variables
required_vars=("DATABASE_URL" "REDIS_URL" "JWT_SECRET")
for var in "${required_vars[@]}"; do
    if [ -z "${!var}" ]; then
        echo -e "${RED}❌ Required environment variable $var is not set${NC}"
        exit 1
    fi
done

# Build and start services
echo -e "${GREEN}🔨 Building and starting services...${NC}"
docker-compose -f $COMPOSE_FILE up --build -d

# Wait for services to be healthy
echo -e "${GREEN}⏳ Waiting for services to be healthy...${NC}"
timeout=300
elapsed=0
while [ $elapsed -lt $timeout ]; do
    if docker-compose -f $COMPOSE_FILE ps | grep -q "unhealthy"; then
        echo -e "${YELLOW}⏳ Services still starting... (${elapsed}s/${timeout}s)${NC}"
        sleep 10
        elapsed=$((elapsed + 10))
    else
        break
    fi
done

# Check if all services are running
if docker-compose -f $COMPOSE_FILE ps | grep -q "unhealthy"; then
    echo -e "${RED}❌ Some services failed to start properly${NC}"
    docker-compose -f $COMPOSE_FILE logs
    exit 1
fi

echo -e "${GREEN}✅ Deployment completed successfully!${NC}"
echo -e "${GREEN}📊 Services status:${NC}"
docker-compose -f $COMPOSE_FILE ps

echo -e "${GREEN}🌐 Application is available at:${NC}"
echo -e "  • API: http://localhost:3000"
echo -e "  • Health: http://localhost:3000/health"
echo -e "  • Docs: http://localhost:3000/docs"
echo -e "  • Metrics: http://localhost:3000/metrics"

if docker-compose -f $COMPOSE_FILE --profile monitoring ps | grep -q "Up"; then
    echo -e "  • Prometheus: http://localhost:9090"
    echo -e "  • Grafana: http://localhost:3001 (admin/admin)"
fi
