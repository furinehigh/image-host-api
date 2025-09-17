#!/bin/bash

# Script to create an API key via the admin endpoint
# Usage: ./create-api-key.sh <admin_jwt_token> <key_name> [daily_limit] [monthly_limit]

set -e

ADMIN_TOKEN=${1:-}
KEY_NAME=${2:-"Default Key"}
DAILY_LIMIT=${3:-1000}
MONTHLY_LIMIT=${4:-30000}
BASE_URL=${BASE_URL:-"http://localhost:3000"}

if [ -z "$ADMIN_TOKEN" ]; then
    echo "Usage: $0 <admin_jwt_token> <key_name> [daily_limit] [monthly_limit]"
    echo "Example: $0 eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9... 'My API Key' 5000 150000"
    exit 1
fi

echo "Creating API key: $KEY_NAME"
echo "Daily limit: $DAILY_LIMIT"
echo "Monthly limit: $MONTHLY_LIMIT"

curl -X POST "$BASE_URL/v1/admin/keys" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{
        \"name\": \"$KEY_NAME\",
        \"daily_limit\": $DAILY_LIMIT,
        \"monthly_limit\": $MONTHLY_LIMIT,
        \"max_images\": 10000,
        \"max_image_size_bytes\": 20971520,
        \"allowed_origins\": [\"*\"]
    }" | jq '.'

echo -e "\n✅ API key created successfully!"
echo "⚠️  Make sure to save the API key from the response above - it won't be shown again!"
