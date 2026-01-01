# =============================================================================
# CACHE-KIT AXUM+GRPC EXAMPLE MAKEFILE
# =============================================================================

CARGO := cargo
DATABASE_URL ?= postgres://postgres:password@localhost:5432/cache_kit
PROTOC ?= /Users/kishore/.local/share/mise/installs/protoc/33.2/bin/protoc

.DEFAULT_GOAL := help

# =============================================================================
# HELP
# =============================================================================

help:  ## Show all available Makefile targets
	@echo "cache-kit Axum+gRPC Example"
	@echo ""
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}' | \
		sort

# =============================================================================
# INFRASTRUCTURE
# =============================================================================

up:  ## Start database
	@echo "Starting PostgreSQL..."
	@docker-compose up -d
	@sleep 3
	@echo "✓ Database started"

down:  ## Stop database
	@echo "Stopping PostgreSQL..."
	@docker-compose down
	@echo "✓ Database stopped"

# =============================================================================
# DEVELOPMENT
# =============================================================================

_fmt:
	@echo "→ Formatting code..."
	@$(CARGO) fmt

_clippy:
	@echo "→ Running clippy..."
	@PROTOC=$(PROTOC) DATABASE_URL=$(DATABASE_URL) $(CARGO) clippy --all-targets -- -D warnings

dev: _fmt _clippy  ## Run formatter and linter (daily workflow)
	@echo ""
	@echo "✓ All checks passed"

# =============================================================================
# BUILDING & RUNNING
# =============================================================================

build:  ## Build the example
	@echo "Building axumgrpc..."
	@PROTOC=$(PROTOC) $(CARGO) build --bin axumgrpc
	@echo "✓ Build complete"

run: up  ## Run the server (requires 'make up')
	@echo "Starting server..."
	@PROTOC=$(PROTOC) DATABASE_URL=$(DATABASE_URL) $(CARGO) run --bin axumgrpc

# =============================================================================
# TESTING
# =============================================================================

test: up  ## Run gRPC cache integration tests (requires 'make up')
	@echo "Running gRPC cache integration tests..."
	@PROTOC=$(PROTOC) DATABASE_URL=$(DATABASE_URL) $(CARGO) test --test grpc_cache_test -- --test-threads=1 --nocapture
	@docker-compose down
	@echo ""
	@echo "========================================="
	@echo "✓ ALL TESTS PASSED"
	@echo "========================================="

.PHONY: help up down build run _fmt _clippy dev test
