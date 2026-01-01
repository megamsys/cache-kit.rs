# =============================================================================
# CACHE-KIT AXUM+METRICS EXAMPLE MAKEFILE
# =============================================================================

CARGO := cargo

.DEFAULT_GOAL := help

# =============================================================================
# HELP
# =============================================================================

help:  ## Show all available Makefile targets
	@echo "cache-kit Axum+Metrics Example"
	@echo ""
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}' | \
		sort

# =============================================================================
# DEVELOPMENT
# =============================================================================

_fmt:
	@echo "→ Formatting code..."
	@$(CARGO) fmt

_clippy:
	@echo "→ Running clippy..."
	@$(CARGO) clippy --all-targets -- -D warnings

dev: _fmt _clippy  ## Run formatter and linter (daily workflow)
	@echo ""
	@echo "✓ All checks passed"

# =============================================================================
# BUILDING & RUNNING
# =============================================================================

build:  ## Build the example
	@echo "Building axummetrics..."
	@$(CARGO) build --bin axummetrics
	@echo "✓ Build complete"

run:  ## Run the server
	@echo "Starting server..."
	@$(CARGO) run --bin axummetrics

.PHONY: help _fmt _clippy dev build run
