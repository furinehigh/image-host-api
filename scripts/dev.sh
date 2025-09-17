#!/bin/bash

set -e

echo "🚀 Starting development environment..."

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi

# Create .env file if it doesn't exist
if [ ! -f ".env" ]; then
    echo "📝 Creating .env file from template..."
    cp .env.example .env
fi

# Start development services
echo "🔨 Starting development services..."
docker-compose -f docker-compose.dev.yml up --build

echo "✅ Development environment started!"
echo "🌐 Application is available at http://localhost:3000"
