# =============================================================================
# QUALITY & TESTING - make/quality.mk
# =============================================================================
# Testing, auditing, and release management

.PHONY: test release _run-tests _audit _publish

# =============================================================================
# PRIVATE HELPERS
# =============================================================================

_run-tests: _check-rust
	@echo "→ Running tests..."
	$(CARGO) test $(FEATURES); \

_audit:
	@echo "→ Running security audit..."
	@command -v cargo-audit >/dev/null 2>&1 || { \
		echo "Installing cargo-audit..."; \
		$(CARGO) install cargo-audit; \
	}
	@$(CARGO) audit

_publish:
	@echo ""
	@echo "========================================="
	@echo "⚠️  Ready to publish to crates.io"
	@echo "========================================="
	@read -p "Publish cache-kit? (yes/no): " confirm && [ "$$confirm" = "yes" ] || exit 1
	@$(CARGO) publish --all-features
	@echo "✓ Published successfully"

# =============================================================================
# PUBLIC TARGETS
# =============================================================================

test: _run-tests  ## Run all tests (use FEATURES="--features redis" to filter)
	@echo ""
	@echo "✓ All tests passed"

release: _check-rust build_release test _audit _publish  ## Build, test, audit, and publish to crates.io
	@echo ""
	@echo "========================================="
	@echo "✓ RELEASE COMPLETE"
	@echo "========================================="
