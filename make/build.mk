# =============================================================================
# BUILD OPERATIONS - make/build.mk
# =============================================================================
# Compilation with feature flag support

.PHONY: build release

# =============================================================================
# PUBLIC TARGETS
# =============================================================================

build: _check-rust  ## Build project in debug mode (use FEATURES="--features redis" or "--all-features")
	@echo "Building cache-kit (debug)..."
	@$(CARGO) build $(FEATURES)
	@echo "✓ Build complete"

build_release: _check-rust  ## Build project in release mode (use FEATURES="--features redis" or "--all-features")
	@echo "Building cache-kit (release)..."
	@$(CARGO) build --release $(FEATURES)
	@echo "✓ Release build complete"
