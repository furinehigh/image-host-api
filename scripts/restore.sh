#!/bin/bash

set -e

if [ $# -ne 1 ]; then
    echo "Usage: $0 <backup_timestamp>"
    echo "Example: $0 20231201_143000"
    exit 1
fi

TIMESTAMP=$1
BACKUP_DIR="./backups"
POSTGRES_CONTAINER="rust-image-hosting-postgres-1"

echo "🔄 Starting restore process for backup: $TIMESTAMP"

# Check if backup files exist
if [ ! -f "$BACKUP_DIR/postgres_$TIMESTAMP.sql" ]; then
    echo "❌ PostgreSQL backup file not found: $BACKUP_DIR/postgres_$TIMESTAMP.sql"
    exit 1
fi

# Stop application to prevent data corruption
echo "⏹️  Stopping application..."
docker-compose stop app

# Restore PostgreSQL database
echo "📦 Restoring PostgreSQL database..."
docker exec -i $POSTGRES_CONTAINER psql -U postgres -c "DROP DATABASE IF EXISTS image_hosting;"
docker exec -i $POSTGRES_CONTAINER psql -U postgres -c "CREATE DATABASE image_hosting;"
docker exec -i $POSTGRES_CONTAINER psql -U postgres image_hosting < "$BACKUP_DIR/postgres_$TIMESTAMP.sql"

# Restore uploaded files
if [ -f "$BACKUP_DIR/uploads_$TIMESTAMP.tar.gz" ]; then
    echo "📁 Restoring uploaded files..."
    rm -rf ./uploads/*
    tar -xzf "$BACKUP_DIR/uploads_$TIMESTAMP.tar.gz"
fi

# Restore Redis data
if [ -f "$BACKUP_DIR/redis_$TIMESTAMP.rdb" ]; then
    echo "💾 Restoring Redis data..."
    docker-compose stop redis
    docker cp "$BACKUP_DIR/redis_$TIMESTAMP.rdb" rust-image-hosting-redis-1:/data/dump.rdb
    docker-compose start redis
fi

# Start application
echo "▶️  Starting application..."
docker-compose start app

echo "✅ Restore completed successfully!"
