# =============================================================================
# DEVELOPMENT OPERATIONS - make/dev.mk
# =============================================================================
# Development workflow: format, lint, documentation

.PHONY: dev doc _fmt _clippy

# =============================================================================
# PRIVATE HELPERS
# =============================================================================

_fmt:
	@echo "→ Formatting code..."
	@$(CARGO) fmt

_clippy:
	@echo "→ Running clippy..."
	@$(CARGO) clippy $(FEATURES) --all-targets -- -D warnings

# =============================================================================
# PUBLIC TARGETS
# =============================================================================

dev: _check-rust _fmt _clippy  ## Run formatter and linter (daily workflow)
	@echo ""
	@echo "✓ All checks passed"

doc: _check-rust  ## Generate documentation
	@echo "Building documentation..."
	@$(CARGO) doc --no-deps --all-features --document-private-items
	@echo "✓ Docs ready: target/doc/cache_kit/index.html"
