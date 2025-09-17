.PHONY: build test run docker-build docker-run clean migrate dev

# Build the application
build:
	cargo build --release

# Run tests
test:
	cargo test

# Run the application in development mode
dev:
	cargo run

# Run the application
run: build
	./target/release/image-hosting-server

# Build Docker image
docker-build:
	docker build -t image-hosting-server .

# Run with Docker Compose
docker-run:
	docker-compose up --build

# Clean build artifacts
clean:
	cargo clean
	docker-compose down -v

# Run database migrations
migrate:
	sqlx migrate run

# Setup development environment
setup-dev:
	docker-compose up -d postgres redis minio
	sleep 5
	sqlx migrate run

# Run integration tests
test-integration:
	docker-compose up -d postgres redis minio
	sleep 5
	sqlx migrate run
	cargo test --test integration_tests
	docker-compose down

# Format code
fmt:
	cargo fmt

# Run clippy linter
lint:
	cargo clippy -- -D warnings

# Generate OpenAPI documentation
docs:
	cargo run --bin generate-openapi > openapi.json

# Performance test with sample images
perf-test:
	./scripts/test-upload.sh $(API_KEY) test-images/sample1.jpg
	./scripts/test-upload.sh $(API_KEY) test-images/sample2.png
	./scripts/test-upload.sh $(API_KEY) test-images/sample3.webp

# Create test API key
create-test-key:
	./scripts/create-api-key.sh $(ADMIN_TOKEN) "Test Key" 10000 300000
