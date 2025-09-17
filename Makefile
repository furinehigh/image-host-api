.PHONY: help build dev prod test clean backup restore logs

# Default target
help:
	@echo "Available commands:"
	@echo "  dev      - Start development environment"
	@echo "  prod     - Deploy production environment"
	@echo "  build    - Build Docker images"
	@echo "  test     - Run tests"
	@echo "  clean    - Clean up Docker resources"
	@echo "  backup   - Create backup of data"
	@echo "  restore  - Restore from backup (usage: make restore TIMESTAMP=20231201_143000)"
	@echo "  logs     - Show application logs"
	@echo "  monitor  - Start with monitoring stack"

# Development environment
dev:
	@chmod +x scripts/dev.sh
	@./scripts/dev.sh

# Production deployment
prod:
	@chmod +x scripts/deploy.sh
	@./scripts/deploy.sh

# Build images
build:
	@docker-compose build

# Run tests
test:
	@docker-compose -f docker-compose.dev.yml run --rm app cargo test

# Clean up
clean:
	@docker-compose down -v
	@docker system prune -f

# Backup
backup:
	@chmod +x scripts/backup.sh
	@./scripts/backup.sh

# Restore
restore:
	@chmod +x scripts/restore.sh
	@./scripts/restore.sh $(TIMESTAMP)

# Show logs
logs:
	@docker-compose logs -f app

# Start with monitoring
monitor:
	@docker-compose --profile monitoring up -d
