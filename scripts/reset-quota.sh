#!/bin/bash

# Script to reset quotas for an API key
# Usage: ./reset-quota.sh <admin_jwt_token> <api_key_id> <quota_type>

set -e

ADMIN_TOKEN=${1:-}
API_KEY_ID=${2:-}
QUOTA_TYPE=${3:-"daily"}
BASE_URL=${BASE_URL:-"http://localhost:3000"}

if [ -z "$ADMIN_TOKEN" ] || [ -z "$API_KEY_ID" ]; then
    echo "Usage: $0 <admin_jwt_token> <api_key_id> [quota_type]"
    echo "quota_type: daily, monthly, all (default: daily)"
    echo "Example: $0 eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9... abc123... monthly"
    exit 1
fi

echo "Resetting $QUOTA_TYPE quota for API key: $API_KEY_ID"

curl -X POST "$BASE_URL/v1/admin/keys/$API_KEY_ID/reset-quota" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"quota_type\": \"$QUOTA_TYPE\"}" | jq '.'

echo -e "\n✅ Quota reset completed!"
