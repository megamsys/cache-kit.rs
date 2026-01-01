# =============================================================================
# PERFORMANCE BENCHMARKS - make/perf.mk
# =============================================================================
# Criterion-based performance benchmarking

.PHONY: perf perf-save perf-diff _bench _open-report

# =============================================================================
# PRIVATE HELPERS
# =============================================================================

_bench:
	@echo "→ Running InMemory benchmarks..."
	@$(CARGO) bench --bench cache_benchmark --no-fail-fast
	@echo ""
	@echo "→ Running Redis benchmarks (requires Redis on localhost:6379)..."
	@$(CARGO) bench --bench redis_benchmark --features redis --no-fail-fast || echo "⚠ Redis benchmarks skipped (Redis not available or feature disabled)"
	@echo ""
	@echo "→ Running Memcached benchmarks (requires Memcached on localhost:11211)..."
	@$(CARGO) bench --bench memcached_benchmark --features memcached --no-fail-fast || echo "⚠ Memcached benchmarks skipped (Memcached not available or feature disabled)"

_open-report:
	@command -v open >/dev/null 2>&1 && open target/criterion/report/index.html || \
		(command -v xdg-open >/dev/null 2>&1 && xdg-open target/criterion/report/index.html) || \
		echo "→ View: target/criterion/report/index.html"

# =============================================================================
# PUBLIC TARGETS
# =============================================================================

perf: _check-rust _bench _open-report  ## Run benchmarks and open HTML report
	@echo ""
	@echo "✓ Benchmarks complete"

perf-save: _check-rust  ## Save current performance as baseline
	@echo "→ Saving InMemory baseline..."
	@$(CARGO) bench --bench cache_benchmark --no-fail-fast -- --save-baseline main
	@echo ""
	@echo "→ Saving Redis baseline (requires Redis on localhost:6379)..."
	@$(CARGO) bench --bench redis_benchmark --features redis --no-fail-fast -- --save-baseline main || echo "⚠ Redis baseline skipped"
	@echo ""
	@echo "→ Saving Memcached baseline (requires Memcached on localhost:11211)..."
	@$(CARGO) bench --bench memcached_benchmark --features memcached --no-fail-fast -- --save-baseline main || echo "⚠ Memcached baseline skipped"
	@echo ""
	@echo "✓ Baselines saved"

perf-diff: _check-rust  ## Compare current performance vs baseline
	@echo "→ Comparing InMemory vs baseline 'main'..."
	@$(CARGO) bench --bench cache_benchmark --no-fail-fast -- --baseline main
	@echo ""
	@echo "→ Comparing Redis vs baseline 'main' (requires Redis on localhost:6379)..."
	@$(CARGO) bench --bench redis_benchmark --features redis --no-fail-fast -- --baseline main || echo "⚠ Redis comparison skipped"
	@echo ""
	@echo "→ Comparing Memcached vs baseline 'main' (requires Memcached on localhost:11211)..."
	@$(CARGO) bench --bench memcached_benchmark --features memcached --no-fail-fast -- --baseline main || echo "⚠ Memcached comparison skipped"
	@echo ""
	@echo "✓ Comparison complete"
