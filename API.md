# Image Hosting Server - API Documentation

Complete API reference for the Rust Image Hosting Server.

## 📋 Table of Contents

- [Authentication](#authentication)
- [Image Management](#image-management)
- [User Management](#user-management)
- [System Endpoints](#system-endpoints)
- [Error Handling](#error-handling)
- [Rate Limiting](#rate-limiting)
- [Examples](#examples)

## 🔐 Authentication

The API supports two authentication methods:

### 1. JWT Bearer Tokens
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

### 2. API Keys
\`\`\`http
Authorization: ApiKey <api_key>
\`\`\`

---

## 🔑 Authentication Endpoints

### Register User
Create a new user account.

**Endpoint:** `POST /api/v1/auth/register`

**Request Body:**
\`\`\`json
{
  "username": "john_doe",
  "email": "john@example.com",
  "password": "secure_password123"
}
\`\`\`

**Response:** `201 Created`
\`\`\`json
{
  "id": "uuid-here",
  "username": "john_doe",
  "email": "john@example.com",
  "quota_used": 0,
  "quota_limit": 1073741824,
  "created_at": "2024-01-15T10:30:00Z"
}
\`\`\`

**Error Responses:**
- `400 Bad Request` - Invalid input data
- `409 Conflict` - Username or email already exists

---

### Login User
Authenticate and receive JWT token.

**Endpoint:** `POST /api/v1/auth/login`

**Request Body:**
\`\`\`json
{
  "email": "john@example.com",
  "password": "secure_password123"
}
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "uuid-here",
    "username": "john_doe",
    "email": "john@example.com"
  }
}
\`\`\`

**Error Responses:**
- `401 Unauthorized` - Invalid credentials
- `400 Bad Request` - Missing required fields

---

### Refresh Token
Refresh an expired JWT token.

**Endpoint:** `POST /api/v1/auth/refresh`

**Headers:**
\`\`\`http
Authorization: Bearer <expired_token>
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600
}
\`\`\`

---

### Create API Key
Generate a new API key for programmatic access.

**Endpoint:** `POST /api/v1/auth/api-keys`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Request Body:**
\`\`\`json
{
  "name": "My App Integration",
  "permissions": ["upload", "read", "delete"]
}
\`\`\`

**Response:** `201 Created`
\`\`\`json
{
  "id": "uuid-here",
  "name": "My App Integration",
  "key": "ak_1234567890abcdef...",
  "permissions": ["upload", "read", "delete"],
  "created_at": "2024-01-15T10:30:00Z",
  "last_used": null
}
\`\`\`

---

### List API Keys
Get all API keys for the authenticated user.

**Endpoint:** `GET /api/v1/auth/api-keys`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "api_keys": [
    {
      "id": "uuid-here",
      "name": "My App Integration",
      "permissions": ["upload", "read", "delete"],
      "created_at": "2024-01-15T10:30:00Z",
      "last_used": "2024-01-15T14:22:00Z"
    }
  ]
}
\`\`\`

---

### Revoke API Key
Delete an API key.

**Endpoint:** `DELETE /api/v1/auth/api-keys/{key_id}`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Response:** `204 No Content`

---

## 🖼️ Image Management

### Upload Image
Upload a new image file.

**Endpoint:** `POST /api/v1/upload`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
Content-Type: multipart/form-data
\`\`\`

**Request Body:**
\`\`\`
Form data:
- file: <image_file>
- alt_text: "Optional description" (optional)
- tags: "tag1,tag2,tag3" (optional)
\`\`\`

**Response:** `201 Created`
\`\`\`json
{
  "id": "uuid-here",
  "filename": "image.jpg",
  "original_filename": "my-photo.jpg",
  "content_type": "image/jpeg",
  "size": 1048576,
  "width": 1920,
  "height": 1080,
  "hash": "sha256:abc123...",
  "alt_text": "Optional description",
  "tags": ["tag1", "tag2", "tag3"],
  "url": "/api/v1/images/uuid-here",
  "thumbnail_url": "/api/v1/images/uuid-here/transform?width=200&height=200",
  "created_at": "2024-01-15T10:30:00Z"
}
\`\`\`

**Error Responses:**
- `400 Bad Request` - Invalid file format or size
- `413 Payload Too Large` - File exceeds size limit
- `507 Insufficient Storage` - User quota exceeded

**Supported Formats:**
- JPEG (.jpg, .jpeg)
- PNG (.png)
- WebP (.webp)
- GIF (.gif)
- TIFF (.tiff, .tif)
- BMP (.bmp)

**Size Limits:**
- Maximum file size: 50MB (configurable)
- Maximum dimensions: 10000x10000 pixels

---

### List Images
Get paginated list of user's images.

**Endpoint:** `GET /api/v1/images`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Query Parameters:**
- `page` (integer, default: 1) - Page number
- `limit` (integer, default: 20, max: 100) - Items per page
- `sort` (string, default: "created_at") - Sort field
- `order` (string, default: "desc") - Sort order (asc/desc)
- `tags` (string) - Filter by tags (comma-separated)
- `search` (string) - Search in filename and alt_text

**Example:**
\`\`\`http
GET /api/v1/images?page=1&limit=10&tags=nature,landscape&sort=created_at&order=desc
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "images": [
    {
      "id": "uuid-here",
      "filename": "image.jpg",
      "original_filename": "sunset.jpg",
      "content_type": "image/jpeg",
      "size": 1048576,
      "width": 1920,
      "height": 1080,
      "alt_text": "Beautiful sunset",
      "tags": ["nature", "landscape"],
      "url": "/api/v1/images/uuid-here",
      "thumbnail_url": "/api/v1/images/uuid-here/transform?width=200&height=200",
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 10,
    "total": 45,
    "pages": 5,
    "has_next": true,
    "has_prev": false
  }
}
\`\`\`

---

### Get Image
Retrieve original image file.

**Endpoint:** `GET /api/v1/images/{image_id}`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token> (optional for public images)
\`\`\`

**Response:** `200 OK`
- Content-Type: Original image MIME type
- Content-Length: File size in bytes
- Cache-Control: public, max-age=31536000
- ETag: File hash for caching

**Error Responses:**
- `404 Not Found` - Image doesn't exist or no access
- `410 Gone` - Image was deleted

---

### Transform Image
Get transformed/resized version of image.

**Endpoint:** `GET /api/v1/images/{image_id}/transform`

**Query Parameters:**
- `width` (integer) - Target width in pixels
- `height` (integer) - Target height in pixels
- `quality` (integer, 1-100) - JPEG quality (default: 85)
- `format` (string) - Output format: jpeg, png, webp (default: original)
- `crop` (string) - Crop mode: fit, fill, crop (default: fit)
- `blur` (float, 0.0-10.0) - Gaussian blur radius
- `sharpen` (float, 0.0-10.0) - Sharpen amount
- `grayscale` (boolean) - Convert to grayscale
- `rotate` (integer) - Rotation angle (90, 180, 270)

**Crop Modes:**
- `fit` - Resize to fit within dimensions, maintaining aspect ratio
- `fill` - Resize to fill dimensions, maintaining aspect ratio (may crop)
- `crop` - Resize and crop to exact dimensions

**Examples:**
\`\`\`http
# Resize to 800x600, maintain aspect ratio
GET /api/v1/images/uuid/transform?width=800&height=600&crop=fit

# Convert to WebP with 90% quality
GET /api/v1/images/uuid/transform?format=webp&quality=90

# Create thumbnail with blur effect
GET /api/v1/images/uuid/transform?width=200&height=200&crop=fill&blur=1.0

# Rotate and convert to grayscale
GET /api/v1/images/uuid/transform?rotate=90&grayscale=true
\`\`\`

**Response:** `200 OK`
- Transformed image data
- Content-Type: Requested format MIME type
- Cache-Control: public, max-age=31536000

**Error Responses:**
- `400 Bad Request` - Invalid transformation parameters
- `404 Not Found` - Image doesn't exist

---

### Get Image Metadata
Retrieve image information without downloading the file.

**Endpoint:** `GET /api/v1/images/{image_id}/info`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "id": "uuid-here",
  "filename": "image.jpg",
  "original_filename": "my-photo.jpg",
  "content_type": "image/jpeg",
  "size": 1048576,
  "width": 1920,
  "height": 1080,
  "hash": "sha256:abc123...",
  "alt_text": "Photo description",
  "tags": ["vacation", "beach"],
  "exif": {
    "camera": "Canon EOS R5",
    "lens": "RF 24-70mm f/2.8L IS USM",
    "focal_length": "50mm",
    "aperture": "f/2.8",
    "shutter_speed": "1/125",
    "iso": 400,
    "date_taken": "2024-01-15T08:30:00Z"
  },
  "created_at": "2024-01-15T10:30:00Z",
  "updated_at": "2024-01-15T10:30:00Z"
}
\`\`\`

---

### Update Image
Update image metadata.

**Endpoint:** `PATCH /api/v1/images/{image_id}`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
Content-Type: application/json
\`\`\`

**Request Body:**
\`\`\`json
{
  "alt_text": "Updated description",
  "tags": ["new-tag", "updated"]
}
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "id": "uuid-here",
  "alt_text": "Updated description",
  "tags": ["new-tag", "updated"],
  "updated_at": "2024-01-15T11:00:00Z"
}
\`\`\`

---

### Delete Image
Permanently delete an image.

**Endpoint:** `DELETE /api/v1/images/{image_id}`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Response:** `204 No Content`

**Error Responses:**
- `404 Not Found` - Image doesn't exist or no access
- `409 Conflict` - Image is referenced elsewhere

---

## 👤 User Management

### Get User Quota
Check current storage usage and limits.

**Endpoint:** `GET /api/v1/user/quota`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "used": 524288000,
  "limit": 1073741824,
  "percentage": 48.8,
  "remaining": 549453824,
  "image_count": 127,
  "breakdown": {
    "images": 520000000,
    "thumbnails": 4288000
  }
}
\`\`\`

---

### Get User Profile
Retrieve user account information.

**Endpoint:** `GET /api/v1/user/profile`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "id": "uuid-here",
  "username": "john_doe",
  "email": "john@example.com",
  "quota_used": 524288000,
  "quota_limit": 1073741824,
  "image_count": 127,
  "api_key_count": 2,
  "created_at": "2024-01-01T00:00:00Z",
  "last_login": "2024-01-15T10:30:00Z"
}
\`\`\`

---

### Update User Profile
Update user account information.

**Endpoint:** `PATCH /api/v1/user/profile`

**Headers:**
\`\`\`http
Authorization: Bearer <jwt_token>
Content-Type: application/json
\`\`\`

**Request Body:**
\`\`\`json
{
  "username": "new_username",
  "email": "new@example.com"
}
\`\`\`

**Response:** `200 OK`
\`\`\`json
{
  "id": "uuid-here",
  "username": "new_username",
  "email": "new@example.com",
  "updated_at": "2024-01-15T11:00:00Z"
}
\`\`\`

---

## 🔧 System Endpoints

### Health Check
Check service health and dependencies.

**Endpoint:** `GET /health`

**Response:** `200 OK`
\`\`\`json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00Z",
  "version": "1.0.0",
  "uptime": 86400,
  "checks": {
    "database": {
      "status": "healthy",
      "response_time": "5ms",
      "connections": {
        "active": 3,
        "idle": 7,
        "max": 20
      }
    },
    "redis": {
      "status": "healthy",
      "response_time": "1ms",
      "memory_usage": "45MB"
    },
    "storage": {
      "status": "healthy",
      "disk_usage": {
        "used": "15GB",
        "available": "85GB",
        "percentage": 15
      }
    }
  }
}
\`\`\`

**Unhealthy Response:** `503 Service Unavailable`

---

### Metrics
Prometheus metrics endpoint.

**Endpoint:** `GET /metrics`

**Response:** `200 OK` (Prometheus format)
\`\`\`
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",status="200"} 1234

# HELP image_processing_duration_seconds Time spent processing images
# TYPE image_processing_duration_seconds histogram
image_processing_duration_seconds_bucket{le="0.1"} 100
image_processing_duration_seconds_bucket{le="0.5"} 450
image_processing_duration_seconds_bucket{le="1.0"} 800
\`\`\`

---

### API Documentation
Interactive API documentation (Swagger UI).

**Endpoint:** `GET /docs`

**Response:** HTML page with interactive API documentation

---

## ⚠️ Error Handling

### Error Response Format
All errors follow a consistent JSON format:

\`\`\`json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input data",
    "details": {
      "field": "email",
      "reason": "Invalid email format"
    },
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req_123456789"
  }
}
\`\`\`

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `VALIDATION_ERROR` | 400 | Invalid request data |
| `AUTHENTICATION_REQUIRED` | 401 | Missing or invalid authentication |
| `INSUFFICIENT_PERMISSIONS` | 403 | User lacks required permissions |
| `RESOURCE_NOT_FOUND` | 404 | Requested resource doesn't exist |
| `RESOURCE_CONFLICT` | 409 | Resource already exists or conflict |
| `PAYLOAD_TOO_LARGE` | 413 | File or request too large |
| `RATE_LIMIT_EXCEEDED` | 429 | Too many requests |
| `QUOTA_EXCEEDED` | 507 | Storage quota exceeded |
| `INTERNAL_ERROR` | 500 | Server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

---

## 🚦 Rate Limiting

### Default Limits
- **Anonymous users**: 10 requests/minute
- **Authenticated users**: 100 requests/minute
- **Upload endpoints**: 20 uploads/hour per user
- **Transform endpoints**: 200 transforms/hour per user

### Rate Limit Headers
All responses include rate limit information:

\`\`\`http
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1642248600
X-RateLimit-Window: 60
\`\`\`

### Rate Limit Exceeded Response
\`\`\`json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "Rate limit exceeded",
    "details": {
      "limit": 100,
      "window": 60,
      "reset_at": "2024-01-15T10:31:00Z"
    }
  }
}
\`\`\`

---

## 📝 Examples

### Complete Upload Workflow

\`\`\`bash
# 1. Register user
curl -X POST http://localhost:3000/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "photographer",
    "email": "photo@example.com",
    "password": "secure123"
  }'

# 2. Login and get token
TOKEN=$(curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "photo@example.com",
    "password": "secure123"
  }' | jq -r '.access_token')

# 3. Upload image
IMAGE_ID=$(curl -X POST http://localhost:3000/api/v1/upload \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@photo.jpg" \
  -F "alt_text=Beautiful landscape" \
  -F "tags=nature,landscape,sunset" | jq -r '.id')

# 4. Get thumbnail
curl "http://localhost:3000/api/v1/images/$IMAGE_ID/transform?width=300&height=200&crop=fill" \
  -H "Authorization: Bearer $TOKEN" \
  -o thumbnail.jpg

# 5. List all images
curl "http://localhost:3000/api/v1/images?tags=nature&limit=10" \
  -H "Authorization: Bearer $TOKEN"
\`\`\`

### JavaScript/Node.js Example

\`\`\`javascript
const API_BASE = 'http://localhost:3000/api/v1';

class ImageHostingClient {
  constructor(token) {
    this.token = token;
  }

  async uploadImage(file, metadata = {}) {
    const formData = new FormData();
    formData.append('file', file);
    
    if (metadata.alt_text) formData.append('alt_text', metadata.alt_text);
    if (metadata.tags) formData.append('tags', metadata.tags.join(','));

    const response = await fetch(`${API_BASE}/upload`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.token}`
      },
      body: formData
    });

    return response.json();
  }

  async getImages(options = {}) {
    const params = new URLSearchParams(options);
    const response = await fetch(`${API_BASE}/images?${params}`, {
      headers: {
        'Authorization': `Bearer ${this.token}`
      }
    });

    return response.json();
  }

  getImageUrl(imageId, transforms = {}) {
    const params = new URLSearchParams(transforms);
    const endpoint = Object.keys(transforms).length > 0 ? 'transform' : '';
    return `${API_BASE}/images/${imageId}${endpoint ? '/' + endpoint : ''}?${params}`;
  }
}

// Usage
const client = new ImageHostingClient('your-jwt-token');

// Upload image
const result = await client.uploadImage(fileInput.files[0], {
  alt_text: 'My photo',
  tags: ['vacation', 'beach']
});

// Get thumbnail URL
const thumbnailUrl = client.getImageUrl(result.id, {
  width: 300,
  height: 200,
  crop: 'fill',
  format: 'webp'
});
\`\`\`

### Python Example

\`\`\`python
import requests
import json

class ImageHostingClient:
    def __init__(self, base_url, token):
        self.base_url = base_url
        self.headers = {'Authorization': f'Bearer {token}'}
    
    def upload_image(self, file_path, alt_text=None, tags=None):
        with open(file_path, 'rb') as f:
            files = {'file': f}
            data = {}
            
            if alt_text:
                data['alt_text'] = alt_text
            if tags:
                data['tags'] = ','.join(tags)
            
            response = requests.post(
                f'{self.base_url}/upload',
                headers=self.headers,
                files=files,
                data=data
            )
            
            return response.json()
    
    def get_images(self, **params):
        response = requests.get(
            f'{self.base_url}/images',
            headers=self.headers,
            params=params
        )
        return response.json()
    
    def get_image_url(self, image_id, **transforms):
        if transforms:
            params = '&'.join([f'{k}={v}' for k, v in transforms.items()])
            return f'{self.base_url}/images/{image_id}/transform?{params}'
        return f'{self.base_url}/images/{image_id}'

# Usage
client = ImageHostingClient('http://localhost:3000/api/v1', 'your-jwt-token')

# Upload image
result = client.upload_image(
    'photo.jpg',
    alt_text='Beautiful sunset',
    tags=['nature', 'landscape']
)

# Get optimized WebP version
webp_url = client.get_image_url(
    result['id'],
    width=800,
    format='webp',
    quality=85
)
\`\`\`

---

## 🔗 SDKs and Libraries

### Official SDKs
- **JavaScript/TypeScript**: `@imagehost/js-sdk`
- **Python**: `imagehost-python`
- **Go**: `github.com/imagehost/go-sdk`
- **PHP**: `imagehost/php-sdk`

### Community Libraries
- **Ruby**: `imagehost-ruby` gem
- **Java**: `imagehost-java` Maven package
- **C#**: `ImageHost.NET` NuGet package

For the latest SDK documentation and examples, visit: https://docs.imagehost.dev/sdks
