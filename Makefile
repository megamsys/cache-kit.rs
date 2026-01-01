# =============================================================================
# CACHE-KIT MAKEFILE - MODULAR STRUCTURE
# =============================================================================
# Main Makefile that orchestrates all operations through modular includes

# =============================================================================
# CONFIGURATION
# =============================================================================
ENVIRONMENT ?= local
CARGO := cargo

# =============================================================================
# COMMON HELPERS
# =============================================================================
.PHONY: _check-rust

_check-rust:
	@command -v $(CARGO) >/dev/null 2>&1 || { echo "Rust/Cargo not found. Install from https://rustup.rs"; exit 1; }

# =============================================================================
# INCLUDE MODULAR MAKEFILES
# =============================================================================
include make/infra.mk
include make/build.mk
include make/dev.mk
include make/quality.mk
include make/perf.mk
include make/release.mk

# =============================================================================
# DEFAULT TARGET
# =============================================================================
.DEFAULT_GOAL := help

# =============================================================================
# HELP
# =============================================================================
help:  ## Show all available Makefile targets
	@echo "Project: cache-kit"
	@echo ""
	@echo "Available Commands:"
	@grep -E '^[a-zA-Z0-9_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		grep -v ':help:' | \
		sed 's/.*\.mk://' | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  %-20s %s\n", $$1, $$2}' | \
		sort
