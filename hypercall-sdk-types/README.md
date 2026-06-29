# hypercall-sdk-types

Shared Rust DTOs for Hypercall SDK clients.

## Contents

- API request and response models.
- WebSocket event models.
- Helper enums and address types.

## Getting Started

Add the type crate from the public Git repository:

```toml
[dependencies]
hypercall-sdk-types = { git = "https://github.com/hypercall-public/hypercall-rust" }
```

Most integrations should use `hypercall-client` instead. Depend on this crate
directly only when you need public request, response, or WebSocket DTOs without
the client runtime.

## Features

- `test-utils`: enables test helpers for downstream crates.

## Compatibility

Serde field names and enum variants are public contracts. Keep changes explicit
and covered by serialization tests.

## Feature Flags

| feature | used by | why |
| --- | --- | --- |
| default | public SDK users | Empty by default. |
| test-utils | tests | Enables shared test constructors and fixtures. |
