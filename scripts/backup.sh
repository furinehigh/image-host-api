#!/bin/bash

set -e

# Configuration
BACKUP_DIR="./backups"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
POSTGRES_CONTAINER="rust-image-hosting-postgres-1"

echo "🗄️  Starting backup process..."

# Create backup directory
mkdir -p $BACKUP_DIR

# Backup PostgreSQL database
echo "📦 Backing up PostgreSQL database..."
docker exec $POSTGRES_CONTAINER pg_dump -U postgres image_hosting > "$BACKUP_DIR/postgres_$TIMESTAMP.sql"

# Backup uploaded files
echo "📁 Backing up uploaded files..."
tar -czf "$BACKUP_DIR/uploads_$TIMESTAMP.tar.gz" ./uploads

# Backup Redis data (optional)
echo "💾 Backing up Redis data..."
docker exec rust-image-hosting-redis-1 redis-cli BGSAVE
sleep 5
docker cp rust-image-hosting-redis-1:/data/dump.rdb "$BACKUP_DIR/redis_$TIMESTAMP.rdb"

echo "✅ Backup completed successfully!"
echo "📂 Backup files:"
ls -la $BACKUP_DIR/*$TIMESTAMP*

# Clean up old backups (keep last 7 days)
find $BACKUP_DIR -name "*.sql" -mtime +7 -delete
find $BACKUP_DIR -name "*.tar.gz" -mtime +7 -delete
find $BACKUP_DIR -name "*.rdb" -mtime +7 -delete

echo "🧹 Old backups cleaned up"
