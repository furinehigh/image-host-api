# Rust Image Hosting Server - Production Setup

A high-performance, production-ready image hosting server built with Rust, featuring advanced image processing, authentication, rate limiting, and comprehensive monitoring.

## 🚀 Quick Production Deployment

### Prerequisites

- **Server**: 2+ CPU cores, 4GB+ RAM, 50GB+ SSD storage
- **Docker**: Docker 20.10+ and Docker Compose 2.0+
- **Domain**: Configured domain with SSL certificate
- **Services**: PostgreSQL 14+, Redis 6+

### 1. Server Setup

```bash
# Clone the repository
git clone <repository-url>
cd rust-image-hosting

# Copy and configure environment
cp .env.example .env
```

### 2. Environment Configuration

Edit `.env` with your production values:

```env
# Database
DATABASE_URL=postgresql://username:password@db:5432/image_hosting
DATABASE_MAX_CONNECTIONS=20

# Redis
REDIS_URL=redis://redis:6379

# Security
JWT_SECRET=your-super-secure-jwt-secret-key-here
JWT_EXPIRY=3600

# Server
PORT=3000
MAX_FILE_SIZE=52428800  # 50MB
UPLOAD_DIR=/app/uploads

# Rate Limiting
RATE_LIMIT_REQUESTS=100
RATE_LIMIT_WINDOW=60

# Monitoring
RUST_LOG=info
ENABLE_METRICS=true

# Optional: Virus Scanning
CLAMAV_HOST=clamav
CLAMAV_PORT=3310
```

### 3. Production Deployment

```bash
# Deploy with Docker Compose
docker-compose up -d

# Check service status
docker-compose ps

# View logs
docker-compose logs -f app
```

### 4. SSL and Reverse Proxy

The included Nginx configuration handles:
- SSL termination
- Static file serving
- Request routing
- Rate limiting
- Security headers

Update `nginx/nginx.conf` with your domain:

```nginx
server_name your-domain.com;
ssl_certificate /etc/ssl/certs/your-domain.crt;
ssl_certificate_key /etc/ssl/private/your-domain.key;
```

### 5. Database Initialization

```bash
# Run database migrations
docker-compose exec app sqlx migrate run

# Create initial admin user (optional)
docker-compose exec app cargo run --bin create-admin
```

## 🔧 Production Configuration

### Performance Tuning

#### Database Optimization
```sql
-- PostgreSQL configuration (postgresql.conf)
shared_buffers = 1GB
effective_cache_size = 3GB
work_mem = 64MB
maintenance_work_mem = 256MB
max_connections = 100
```

#### Redis Configuration
```conf
# redis.conf
maxmemory 512mb
maxmemory-policy allkeys-lru
save 900 1
save 300 10
save 60 10000
```

#### Application Tuning
```env
# Worker threads (CPU cores * 2)
TOKIO_WORKER_THREADS=4

# Database connection pool
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5

# File upload limits
MAX_FILE_SIZE=52428800  # 50MB
MAX_CONCURRENT_UPLOADS=10
```

### Security Hardening

#### 1. Firewall Configuration
```bash
# UFW firewall rules
ufw allow 22/tcp    # SSH
ufw allow 80/tcp    # HTTP
ufw allow 443/tcp   # HTTPS
ufw deny 5432/tcp   # PostgreSQL (internal only)
ufw deny 6379/tcp   # Redis (internal only)
ufw enable
```

#### 2. SSL/TLS Setup
```bash
# Let's Encrypt with Certbot
certbot --nginx -d your-domain.com
```

#### 3. Security Headers
Already configured in `nginx/nginx.conf`:
- HSTS
- Content Security Policy
- X-Frame-Options
- X-Content-Type-Options

### Monitoring and Logging

#### 1. Prometheus Metrics
Access metrics at: `https://your-domain.com/metrics`

Key metrics monitored:
- Request latency and throughput
- Image processing performance
- Database connection pool status
- Redis cache hit rates
- File upload/download bandwidth

#### 2. Log Management
```bash
# View application logs
docker-compose logs -f app

# View Nginx access logs
docker-compose logs -f nginx

# Export logs to external system
docker-compose logs --no-color app | logger -t image-hosting
```

#### 3. Health Monitoring
```bash
# Health check endpoint
curl https://your-domain.com/health

# Automated monitoring with cron
*/5 * * * * curl -f https://your-domain.com/health || echo "Service down" | mail admin@domain.com
```

## 🔄 Maintenance

### Backup Strategy

#### 1. Database Backups
```bash
# Daily automated backup
./scripts/backup.sh

# Manual backup
docker-compose exec db pg_dump -U username image_hosting > backup_$(date +%Y%m%d).sql
```

#### 2. File Storage Backups
```bash
# Sync uploads to S3/backup location
rsync -av ./uploads/ backup-server:/backups/uploads/
```

### Updates and Deployment

#### 1. Zero-Downtime Updates
```bash
# Pull latest changes
git pull origin main

# Build new image
docker-compose build app

# Rolling update
docker-compose up -d --no-deps app
```

#### 2. Database Migrations
```bash
# Run migrations during maintenance window
docker-compose exec app sqlx migrate run
```

### Scaling Considerations

#### Horizontal Scaling
- Load balancer (HAProxy/Nginx)
- Multiple app instances
- Shared file storage (NFS/S3)
- Database read replicas

#### Vertical Scaling
- Increase server resources
- Optimize database queries
- Implement caching layers
- CDN integration

## 🚨 Troubleshooting

### Common Issues

#### High Memory Usage
```bash
# Check memory usage
docker stats

# Optimize image processing
# Reduce concurrent workers in config
```

#### Database Connection Issues
```bash
# Check database connectivity
docker-compose exec app pg_isready -h db

# Monitor connection pool
curl https://your-domain.com/metrics | grep db_connections
```

#### File Upload Failures
```bash
# Check disk space
df -h

# Verify upload directory permissions
ls -la uploads/

# Check file size limits
grep MAX_FILE_SIZE .env
```

### Performance Monitoring

#### Key Performance Indicators
- Response time < 200ms (95th percentile)
- Image processing < 2s for standard transforms
- Database query time < 50ms average
- Cache hit rate > 80%
- Uptime > 99.9%

#### Alerting Thresholds
```bash
# Set up alerts for:
# - Response time > 500ms
# - Error rate > 1%
# - Disk usage > 80%
# - Memory usage > 90%
# - Database connections > 80% of pool
```

## 📞 Support

For production issues:
1. Check service status: `docker-compose ps`
2. Review logs: `docker-compose logs -f`
3. Monitor metrics: `https://your-domain.com/metrics`
4. Verify health: `https://your-domain.com/health`

For detailed API documentation, see [API.md](./API.md).
