#!/bin/bash

# Script to monitor API key quotas and usage
# Usage: ./monitor-quotas.sh <admin_jwt_token> [api_key_id]

set -e

ADMIN_TOKEN=${1:-}
API_KEY_ID=${2:-}
BASE_URL=${BASE_URL:-"http://localhost:3000"}

if [ -z "$ADMIN_TOKEN" ]; then
    echo "Usage: $0 <admin_jwt_token> [api_key_id]"
    echo "Example: $0 eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9... abc123..."
    exit 1
fi

if [ -n "$API_KEY_ID" ]; then
    echo "Checking quota status for API key: $API_KEY_ID"
    curl -s -X GET "$BASE_URL/v1/admin/keys/$API_KEY_ID/quota" \
        -H "Authorization: Bearer $ADMIN_TOKEN" | jq '.'
else
    echo "Getting usage statistics for all keys..."
    curl -s -X GET "$BASE_URL/v1/admin/usage" \
        -H "Authorization: Bearer $ADMIN_TOKEN" | jq '.'
fi
