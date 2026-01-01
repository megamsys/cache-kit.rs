# Golden Blob Test Files

This directory contains **golden blob files** - reference serialized cache entries used for regression testing.

## Purpose

Golden blobs ensure **serialization format stability** across code changes:

- ✅ **Backward compatibility**: New code can read old cache entries
- ✅ **Accidental change detection**: Refactoring doesn't break serialization
- ✅ **Version migration validation**: Schema changes are intentional

## Files

| File             | Schema Version | Description                                    |
| ---------------- | -------------- | ---------------------------------------------- |
| `user_v1.bin`    | 1              | Serialized User entity (id: 42, name: "Alice") |
| `product_v1.bin` | 1              | Serialized Product entity (id: "prod_123")     |
| `complex_v1.bin` | 1              | Serialized ComplexEntity with collections      |

## Workflow

### Normal Operation

When refactoring code without changing the schema, golden blob tests should continue to pass. If a test fails unexpectedly, it indicates an accidental serialization format change (e.g., field reordering).

```rust
// Example: Reordering fields accidentally changes serialization
#[derive(Serialize, Deserialize)]
struct User {
    name: String,  // Reordered
    id: u64,
}
// ❌ Golden blob test FAILS!
```

### Schema Changes

When intentionally changing the schema (adding/removing fields, changing types, upgrading Postcard):

1. **Bump the schema version** in `src/serialization/mod.rs`:

   ```rust
   pub const CURRENT_SCHEMA_VERSION: u32 = 2;  // Was 1
   ```

2. **Regenerate golden blobs**:

   ```bash
   cargo test --test golden_blob_generator -- --nocapture
   ```

3. **Verify tests pass**:

   ```bash
   cargo test --test golden_blobs
   ```

4. **Old cache entries** (v1) will be automatically evicted in production

### When to Regenerate

**Regenerate when:**

- ✅ Added/removed struct fields
- ✅ Changed field types
- ✅ Upgraded Postcard version
- ✅ Changed serialization logic

**Do not regenerate for:**

- ❌ Refactoring (renaming variables)
- ❌ Code cleanup
- ❌ Documentation updates
- ❌ Non-schema changes

## Production Impact

When you bump the schema version and regenerate golden blobs:

1. **Deploy new code** with bumped `CURRENT_SCHEMA_VERSION`
2. **Old cache entries** are rejected (version mismatch)
3. **Cache misses** trigger database reads
4. **New cache entries** are written with new version
5. **Gradual migration** - no manual cache flush needed

### Monitoring During Deployment

Watch these metrics:

```
cache.version_mismatch # Should spike temporarily
cache.hit_rate # Will drop then recover
cache.miss_count # Will spike then normalize
```

## Checklist

Before committing schema changes:

- [ ] Bumped `CURRENT_SCHEMA_VERSION` in `src/serialization/mod.rs`
- [ ] Regenerated golden blobs: `cargo test --test golden_blob_generator`
- [ ] Verified tests pass: `cargo test --test golden_blobs`
- [ ] Updated CHANGELOG with cache invalidation note
- [ ] Documented schema change in PR description
- [ ] Planned for cache hit rate drop during rollout

## References

- Serialization format: [cachekit.org/serialization](https://cachekit.org/serialization)
- Serialization code: `src/serialization/mod.rs`
- Test code: `tests/golden_blobs.rs`
- Generator: `tests/golden_blob_generator.rs`
