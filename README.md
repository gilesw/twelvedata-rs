# twelvedata-api

Unofficial typed async Rust client for [Twelve Data](https://twelvedata.com).
Generated API bindings for every documented endpoint plus a thin convenience client.
The crate is named `twelvedata-api` because `twelvedata` and `twelve-data` are
other authors' crates; import it as `twelvedata_api`.

The low-level client and response types are generated from Twelve Data's official
[OpenAPI 3.1 specification](https://api.twelvedata.com/doc/swagger/openapi.json) with
[openapi-to-rust](https://github.com/gpu-cli/openapi-to-rust) 0.19.0. A thin wrapper
adds short method names for the common endpoints, batch quotes, header
authentication and shared errors that understand the provider's error bodies.
No application models, symbol mappings, request pacing, retries or caching are included.

## Usage

```toml
[dependencies]
twelvedata-api = "0.1"
```

```rust,no_run
use twelvedata_api::{Client, Interval, TimeSeriesQuery};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let client = Client::new("your-api-key");
let quote = client.quote("AAPL").await?;
println!("{} closed at {} {}", quote.symbol, quote.close, quote.currency.unwrap_or_default());

let series = client
    .time_series(&TimeSeriesQuery {
        outputsize: Some(250),
        ..TimeSeriesQuery::new("VOD.L", Interval::Variant1day)
    })
    .await?;
for bar in series.values {
    println!("{}: {}", bar.datetime, bar.close);
}

for (symbol, quote) in client.quotes(&["AAPL", "MSFT"]).await? {
    match quote {
        Ok(quote) => println!("{symbol}: {}", quote.close),
        Err(message) => println!("{symbol}: {message}"),
    }
}
# Ok(())
# }
```

`cargo run --example quote -- AAPL` reads an API key from standard input.
Running that example contacts the provider and uses your account's credits.
Ordinary tests use local mock servers and need no API key.

| Method | Documented endpoint |
|---|---|
| `quote` | `/quote?symbol=…` |
| `quotes` | `/quote?symbol=A,B,…` (object keyed by symbol) |
| `time_series` | `/time_series?symbol=…&interval=…` |
| `price` | `/price?symbol=…` |
| `eod` | `/eod?symbol=…` |
| `exchange_rate` | `/exchange_rate?symbol=…` |
| `profile` | `/profile?symbol=…` |
| `statistics` | `/statistics?symbol=…` |
| `price_target` | `/price_target?symbol=…` |
| `earnings` | `/earnings?symbol=…` |
| `symbol_search` | `/symbol_search?symbol=…` |
| `api_usage` | `/api_usage` |

`Quote`, `TimeSeries`, `Profile`, `Statistics`, `Interval` and the other short
names are aliases for the generated types. Every generated type is re-exported
at the crate root. Use `client.raw()` for all 187 generated operations,
including the technical indicators, ETF, fund and reference-data endpoints,
with every query parameter and typed per-operation errors.

## Behaviour

- Authentication sends `Authorization: apikey <key>`; the key never appears in
  a URL. `Client` debug output hides the key. `with_base_url` supports mock
  servers and trusted proxies.
- Responses are bounded to 8 MiB by default, configurable through
  `with_max_response_body_bytes`.
- Twelve Data reports errors as `{"code", "message", "status": "error"}`, with
  a matching HTTP status or inside an HTTP 200. Both become `Error::Api` with the
  HTTP status, the provider code and message, the full body and any
  `Retry-After` header. `is_unauthorized`, `is_rate_limited` and `is_not_found`
  check the provider code when present, otherwise the HTTP status. A success
  response that does not match the schema is `Error::Decode`. There are no
  automatic retries or background requests.
- Batch quotes return one entry per symbol as the provider echoes it. A
  rejected symbol maps to its `ApiMessage` instead of failing the batch. A
  whole-request failure, such as a bad key, is an `Error::Api`. A single-symbol
  batch uses the single-quote shape, so its failures are also `Error::Api`.
- Prices, changes and volumes arrive as decimal strings exactly as published.
  The client does not parse, round or convert them, and does not convert
  currencies or units such as `GBp` pence.
- Bar datetimes are in the exchange's local time unless `timezone` is given.
  The provider omits bars without trades; the client never fills gaps.
- The schema's required fields are kept required. A response missing one is
  reported as `Error::Decode` rather than defaulted.

## Layout and regeneration

- `specs/twelvedata.json`: unmodified provider specification retrieved 2026-10-04.
- `openapi-to-rust.toml`: generator configuration and header authentication.
- `src/generated/`: checked-in generator output, skipped by rustfmt and never
  edited by hand. `effective.json` records the input used for generation.
  `src/lib.rs` declares the generated files; the generated `mod.rs` is retained
  as an artifact but not used.
- `src/client.rs`, `src/error.rs`: handwritten convenience layer.
- `tests/fixtures/`: response bodies assembled by `scripts/fixtures.py` from
  the specification's per-property examples. The specification has no complete
  response examples. Error and boundary fixtures in the tests are synthetic.
- `mise.toml`: Rust 1.98.1, pinned linters and generator install/generate/check
  tasks, for reproducible builds. The generator is installed into its own
  `target/tools` prefix.

No schema overlay is currently necessary. Add an overlay only when evidence
shows a discrepancy; retain the original specification unchanged.

```sh
mise run generate
mise run check
```

With `openapi-to-rust` 0.19.0 already installed, the equivalent checks are:

```sh
openapi-to-rust generate --config openapi-to-rust.toml --check
cargo +1.98.1 fmt --all -- --check
cargo +1.98.1 clippy --all-targets --locked -- -D warnings
cargo +1.98.1 test --locked
```

To update the upstream specification deliberately:

```sh
curl --fail --location https://api.twelvedata.com/doc/swagger/openapi.json --output specs/twelvedata.json
mise run generate
python3 scripts/fixtures.py
```

Review the spec diff and the regenerated fixtures before running the checks.
The initial specification SHA-256 is
`ddc6642016d9403b25cf538c74dbcef5c4f814128fd74ec283a50b1a1ef9ce84`.

## Verification and integration status

The HTTP tests use fixtures built from the specification's property examples
and prove agreement with that specification. No endpoint has yet been verified
against a live account. The `quote` example exists for that check.

The specification declares fields such as `name`, `datetime`, `fifty_two_week`
and `is_market_open` as required on `/quote`. If a live response for some
instrument type omits one, that is evidence for a schema overlay relaxing it.

## License and publication

See [RELEASING.md](RELEASING.md) for the commit, tag, verification and crates.io
publishing workflow. Run `mise run release` for a dry run, then
`mise run release:publish` to upload. Both detect the version tag at HEAD and
check that it matches `Cargo.toml`.

The handwritten wrapper is dual-licensed under
[MIT](LICENSE-MIT) and [Apache-2.0](LICENSE-APACHE).
The upstream specification declares no licence. The licence position for
redistributing the specification and derived artifacts remains unresolved. The
manifest permits publication to crates.io; the included licence files do not
relicense the provider's material or data.
