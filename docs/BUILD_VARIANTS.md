# Vision Node Build (Single World)

This repository ships **one build world**: **FULL**.

- Deprecated alternate builds are disabled at compile time.
- The binary you ship should always be produced with the same feature surface.

## Quick Commands

```powershell
# Dev
cargo build
cargo run

# Release
cargo build --release

# Verify
cargo check
cargo test
```

## Optional Build Features

Most operators should **not** enable extra compile-time features. If you do, be intentional about it and treat it as a different build artifact.

- `hwloc`: optional NUMA/topology support (platform-dependent; not enabled by default)

## Notes

- Runtime flags and environment variables must not change chain identity/consensus.
- If you attempt to build without the `full` feature, the build will fail by design.
