# Changelog

## Unreleased

- Require Rust 1.98.1 and add GitHub Actions lint, test and release dry-run workflows.

## 0.1.1

- Keep five keywords so crates.io accepts the manifest. The v0.1.0 tag was
  never published.

## 0.1.0 — repository tag

- Generated client for all 187 documented Twelve Data endpoints.
- Convenience methods for quotes, batch quotes, time series, prices,
  end-of-day closes, exchange rates, fundamentals, symbol search and usage.
- Errors that read the provider's `code`/`message` bodies, including those
  sent with HTTP 200.
- Tests using fixtures built from the specification's property examples.
- Tag-based release script, with dry runs by default.
