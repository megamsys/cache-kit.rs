# =============================================================================
# CACHE-KIT ACTIX+SQLX EXAMPLE MAKEFILE
# =============================================================================

CARGO := cargo
DATABASE_URL ?= postgres://cachekit:cachekit_dev@localhost:5432/cachekit_actix

.DEFAULT_GOAL := help

# =============================================================================
# HELP
# =============================================================================

help:  ## Show all available Makefile targets
	@echo "cache-kit Actix+SQLx Example"
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
	@SQLX_OFFLINE=true $(CARGO) clippy --all-targets -- -D warnings

dev: _fmt _clippy  ## Run formatter and linter (daily workflow)
	@echo ""
	@echo "✓ All checks passed"

# =============================================================================
# TESTING
# =============================================================================

test: up  ## Run tests (requires 'make up')
	@echo "Running tests..."
	@$(CARGO) test -- --test-threads=1
	@docker-compose down
	@echo ""
	@echo "========================================="
	@echo "✓ ALL TESTS PASSED"
	@echo "========================================="

.PHONY: help up down _fmt _clippy dev test
