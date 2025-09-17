#!/bin/bash

# Setup SSL certificates for api.i.ex.tech
# Run this script after starting docker-compose

echo "Setting up SSL certificates for api.i.ex.tech..."

# Wait for nginx to be ready
echo "Waiting for nginx to start..."
sleep 10

# Get initial certificate
docker-compose exec certbot certbot certonly \
    --webroot \
    --webroot-path=/var/www/certbot \
    --email your-email@example.com \
    --agree-tos \
    --no-eff-email \
    -d api.i.ex.tech

# Reload nginx to use the new certificates
docker-compose exec nginx nginx -s reload

echo "SSL setup complete! Your API is now available at https://api.i.ex.tech"
