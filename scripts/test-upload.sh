#!/bin/bash

# Script to test image upload
# Usage: ./test-upload.sh <api_key> <image_file>

set -e

API_KEY=${1:-}
IMAGE_FILE=${2:-}
BASE_URL=${BASE_URL:-"http://localhost:3000"}

if [ -z "$API_KEY" ] || [ -z "$IMAGE_FILE" ]; then
    echo "Usage: $0 <api_key> <image_file>"
    echo "Example: $0 ik_abc123... /path/to/image.jpg"
    exit 1
fi

if [ ! -f "$IMAGE_FILE" ]; then
    echo "Error: Image file '$IMAGE_FILE' not found!"
    exit 1
fi

echo "Uploading image: $IMAGE_FILE"
echo "Using API key: ${API_KEY:0:10}..."

curl -X POST "$BASE_URL/v1/uploads" \
    -H "x-api-key: $API_KEY" \
    -F "file=@$IMAGE_FILE" \
    -F "visibility=public" \
    -F "resize=256" \
    -F "resize=512" \
    -F "resize=1024" | jq '.'

echo -e "\n✅ Image uploaded successfully!"
