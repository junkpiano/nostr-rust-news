# Agent Notes

## Language policy
- Use English for all entries and updates in this file.

## Networking / Reddit fetch policy
- For `reqwest` access to Reddit endpoints, prefer `native-tls` over `rustls` to reduce TLS fingerprint mismatches observed in CI cross-compile artifacts.
- Keep `default-features = false` and use `features = ["json", "native-tls"]` in `Cargo.toml`.
- If Reddit starts returning `403` again, keep `.http1_only()` enabled and log non-2xx response body head for diagnostics.
