# =============================================================================
# INFRASTRUCTURE OPERATIONS - make/infra.mk
# =============================================================================
# Docker infrastructure management for cache backends (Redis, Memcached)

.PHONY: up down _check-docker _wait-services

# Configuration
COMPOSE_INFRA := docker-compose.yml
REDIS_HOST ?= localhost
REDIS_PORT ?= 6379
MEMCACHED_HOST ?= localhost
MEMCACHED_PORT ?= 11211

# Environment variables for testing in tests/redis_integration_test.rs and tests/memcached_integration_test.rs
export TEST_REDIS_URL ?= redis://$(REDIS_HOST):$(REDIS_PORT)
export TEST_MEMCACHED_URL ?= $(MEMCACHED_HOST):$(MEMCACHED_PORT)

# =============================================================================
# PRIVATE HELPERS
# =============================================================================

_check-docker:
	@docker info >/dev/null 2>&1 || { echo "Docker not running. Start Docker Desktop."; exit 1; }

_wait-services:
	@echo "Waiting for services to be healthy..."
	@sleep 15
	@echo "Services ready"

# =============================================================================
# PUBLIC TARGETS
# =============================================================================

up: _check-docker  ## Start Redis and Memcached services
	@docker-compose -f $(COMPOSE_INFRA) up -d
	@$(MAKE) _wait-services
	@echo "✓ Services ready: redis://$(REDIS_HOST):$(REDIS_PORT), memcached://$(MEMCACHED_HOST):$(MEMCACHED_PORT)"

down:  ## Stop all services and clean up
	@docker-compose -f $(COMPOSE_INFRA) down -v 2>/dev/null || true
	@echo "✓ Services stopped"
