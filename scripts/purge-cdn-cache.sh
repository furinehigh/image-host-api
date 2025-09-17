#!/bin/bash

# Script to purge Cloudflare cache for specific URLs
# Usage: ./purge-cdn-cache.sh <zone_id> <api_token> <url1> [url2] [url3]...

set -e

ZONE_ID=${1:-}
API_TOKEN=${2:-}
shift 2
URLS=("$@")

if [ -z "$ZONE_ID" ] || [ -z "$API_TOKEN" ] || [ ${#URLS[@]} -eq 0 ]; then
    echo "Usage: $0 <zone_id> <api_token> <url1> [url2] [url3]..."
    echo "Example: $0 abc123... def456... https://example.com/image1.jpg https://example.com/image2.jpg"
    exit 1
fi

echo "Purging ${#URLS[@]} URLs from Cloudflare cache..."

# Build JSON array of URLs
URLS_JSON=$(printf '%s\n' "${URLS[@]}" | jq -R . | jq -s .)

curl -X POST "https://api.cloudflare.com/client/v4/zones/$ZONE_ID/purge_cache" \
    -H "Authorization: Bearer $API_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"files\": $URLS_JSON}" | jq '.'

echo -e "\n✅ Cache purge request submitted!"
