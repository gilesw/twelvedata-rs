//! Typed async client for Twelve Data, generated from its published OpenAPI spec.
//!
//! ```no_run
//! use twelvedata_api::{Client, Interval, TimeSeriesQuery};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = Client::new("your-api-key");
//! let quote = client.quote("AAPL").await?;
//! println!("{} closed at {} {}", quote.symbol, quote.close, quote.currency.unwrap_or_default());
//!
//! let series = client
//!     .time_series(&TimeSeriesQuery::new("AAPL", Interval::Variant1day))
//!     .await?;
//! for bar in series.values {
//!     println!("{}: {}", bar.datetime, bar.close);
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod client;
mod error;

// Keep generator output byte-identical for --check. The generated mod.rs is
// retained as an artifact but this declaration controls its lint/format scope.
#[rustfmt::skip]
#[allow(
    dead_code,
    non_camel_case_types,
    clippy::all,
    clippy::pedantic,
    missing_docs,
    rustdoc::all
)]
pub mod generated {
    pub mod client;
    pub mod types;
}

pub use client::{BASE_URL, Client, TimeSeriesQuery};
pub use error::{ApiMessage, Error};
pub use generated::types::*;
pub use generated::types::{
    GetApiUsage200Response as ApiUsage, GetEarnings200Response as Earnings,
    GetEod200Response as EndOfDay, GetExchangeRate200Response as ExchangeRate,
    GetPrice200Response as Price, GetPriceTarget200Response as PriceTarget,
    GetProfile200Response as Profile, GetQuote200Response as Quote,
    GetQuote200ResponseFiftyTwoWeek as FiftyTwoWeek, GetStatistics200Response as Statistics,
    GetSymbolSearch200Response as SymbolSearch, GetTimeSeries200Response as TimeSeries,
    GetTimeSeries200ResponseMeta as TimeSeriesMeta, IntervalEnum as Interval, OrderEnum as Order,
};
