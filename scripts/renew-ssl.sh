#!/bin/bash

# Renew SSL certificates
echo "Renewing SSL certificates..."

docker-compose exec certbot certbot renew --quiet
docker-compose exec nginx nginx -s reload

echo "SSL renewal complete!"
