# =============================================================================
# RELEASE OPERATIONS - make/release.mk
# =============================================================================

.PHONY: version-bump

version-bump:  ## Bump version everywhere (usage: make version-bump VERSION=0.10.0)
	@if [ -z "$(VERSION)" ]; then \
		echo "Usage: make version-bump VERSION=0.10.0"; \
		exit 1; \
	fi
	@OLD_VERSION=$$(cat VERSION); \
	OLD_SEMVER=$$(echo $$OLD_VERSION | cut -d. -f1,2); \
	NEW_SEMVER=$$(echo $(VERSION) | cut -d. -f1,2); \
	echo "$(VERSION)" > VERSION; \
	sed -i.bak 's/^version = ".*"/version = "$(VERSION)"/' Cargo.toml && rm Cargo.toml.bak; \
	find docs -name "*.md" -type f -exec sed -i.bak "s/$$OLD_VERSION/$(VERSION)/g" {} \; -exec rm {}.bak \;; \
	find docs -name "*.md" -type f -exec sed -i.bak "s/\"$$OLD_SEMVER\"/\"$$NEW_SEMVER\"/g" {} \; -exec rm {}.bak \;; \
	sed -i.bak "s/$$OLD_VERSION/$(VERSION)/g" README.md && rm README.md.bak; \
	sed -i.bak "s/\"$$OLD_SEMVER\"/\"$$NEW_SEMVER\"/g" README.md && rm README.md.bak; \
	echo "✓ Bumped $$OLD_VERSION → $(VERSION) (semver: $$OLD_SEMVER → $$NEW_SEMVER)"; \
	echo "  Updated: VERSION, Cargo.toml, README.md, docs/**/*.md"; \
	echo "  Next: Update CHANGELOG.md, commit, tag v$(VERSION)"
