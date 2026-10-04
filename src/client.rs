use std::collections::BTreeMap;

use serde_json::Value;

use crate::generated::client::{HttpClient, HttpError};
use crate::{
    ApiMessage, ApiUsage, Earnings, EndOfDay, Error, ExchangeRate, Interval, Order, Price,
    PriceTarget, Profile, Quote, Statistics, SymbolSearch, TimeSeries,
};

pub const BASE_URL: &str = "https://api.twelvedata.com";

const NONE: Option<&str> = None;

/// Parameters for `/time_series`. Dates use the provider's `YYYY-MM-DD` or
/// `YYYY-MM-DD HH:MM:SS` formats, interpreted in `timezone` when given.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TimeSeriesQuery {
    pub symbol: String,
    pub interval: Interval,
    /// Number of bars, 1 to 5000. The provider default is 30.
    pub outputsize: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub exchange: Option<String>,
    pub mic_code: Option<String>,
    pub timezone: Option<String>,
    /// Bar order; the provider default is newest first.
    pub order: Option<Order>,
}

impl TimeSeriesQuery {
    pub fn new(symbol: impl Into<String>, interval: Interval) -> Self {
        Self {
            symbol: symbol.into(),
            interval,
            ..Self::default()
        }
    }
}

/// Authenticated client sending `Authorization: apikey <key>`.
#[derive(Clone)]
pub struct Client {
    inner: HttpClient,
    http: reqwest::Client,
    base_url: String,
    authorization: String,
    max_response_body_bytes: usize,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl Client {
    pub fn new(api_key: impl Into<String>) -> Self {
        let authorization = format!("apikey {}", api_key.into());
        Self {
            inner: HttpClient::new().with_api_key(authorization.clone()),
            http: reqwest::Client::new(),
            base_url: BASE_URL.to_owned(),
            authorization,
            max_response_body_bytes: 8 * 1024 * 1024,
        }
    }

    /// Override the server for testing or a trusted proxy.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self.inner = self.inner.with_base_url(self.base_url.clone());
        self
    }

    /// Bound the bytes read from any response. The default is 8 MiB.
    pub fn with_max_response_body_bytes(mut self, limit: usize) -> Self {
        self.max_response_body_bytes = limit;
        self.inner = self.inner.with_max_response_body_bytes(limit);
        self
    }

    /// Access all generated operations, every query parameter and typed errors.
    pub fn raw(&self) -> &HttpClient {
        &self.inner
    }

    /// Latest quote for one symbol. Prices arrive as decimal strings.
    pub async fn quote(&self, symbol: &str) -> Result<Quote, Error> {
        Ok(self
            .inner
            .get_quote(
                Some(symbol),
                NONE,
                NONE,
                NONE,
                None,
                NONE,
                NONE,
                NONE,
                None,
                None,
                None,
                NONE,
                None,
                None,
                None,
                None,
                NONE,
            )
            .await?)
    }

    /// Quotes for several symbols in one request, keyed by symbol as the
    /// provider echoes it. A symbol the provider rejects maps to its message
    /// rather than failing the batch. With one symbol this behaves as [`Client::quote`].
    pub async fn quotes(
        &self,
        symbols: &[impl AsRef<str>],
    ) -> Result<BTreeMap<String, Result<Quote, ApiMessage>>, Error> {
        let symbols: Vec<&str> = symbols.iter().map(AsRef::as_ref).collect();
        match symbols.as_slice() {
            [] => return Err(Error::InvalidRequest("quotes requires at least one symbol")),
            [symbol] => {
                let quote = self.quote(symbol).await?;
                return Ok(BTreeMap::from([(symbol.to_string(), Ok(quote))]));
            }
            _ => {}
        }
        let body = self
            .get_json("/quote", &[("symbol", symbols.join(","))])
            .await?;
        let Value::Object(entries) = body else {
            return Err(Error::Decode(
                "batch quote response is not an object keyed by symbol".to_owned(),
            ));
        };
        entries
            .into_iter()
            .map(|(symbol, entry)| {
                let quote = match ApiMessage::from_value(&entry) {
                    Some(message) => Err(message),
                    None => Ok(serde_json::from_value(entry)
                        .map_err(|e| Error::Decode(format!("{symbol}: {e}")))?),
                };
                Ok((symbol, quote))
            })
            .collect()
    }

    /// OHLCV bars. Prices arrive as decimal strings in the exchange's local time.
    pub async fn time_series(&self, query: &TimeSeriesQuery) -> Result<TimeSeries, Error> {
        if query.symbol.trim().is_empty() {
            return Err(Error::InvalidRequest("time_series requires a symbol"));
        }
        Ok(self
            .inner
            .get_time_series(
                Some(query.symbol.as_str()),
                NONE,
                NONE,
                NONE,
                query.interval.clone(),
                query.outputsize,
                query.exchange.as_deref(),
                query.mic_code.as_deref(),
                NONE,
                None,
                query.timezone.as_deref(),
                query.start_date.as_deref(),
                query.end_date.as_deref(),
                NONE,
                query.order.clone(),
                None,
                None,
                NONE,
                None,
                None,
                None,
            )
            .await?)
    }

    /// Latest price only, as a decimal string.
    pub async fn price(&self, symbol: &str) -> Result<Price, Error> {
        Ok(self
            .inner
            .get_price(
                Some(symbol),
                NONE,
                NONE,
                NONE,
                NONE,
                NONE,
                NONE,
                None,
                None,
                NONE,
                None,
                None,
            )
            .await?)
    }

    /// Latest end-of-day close.
    pub async fn eod(&self, symbol: &str) -> Result<EndOfDay, Error> {
        Ok(self
            .inner
            .get_eod(
                Some(symbol),
                NONE,
                NONE,
                NONE,
                NONE,
                NONE,
                NONE,
                None,
                NONE,
                None,
                None,
            )
            .await?)
    }

    /// Current rate for a currency pair such as `GBP/USD`.
    pub async fn exchange_rate(&self, symbol: &str) -> Result<ExchangeRate, Error> {
        Ok(self
            .inner
            .get_exchange_rate(symbol, NONE, None, NONE, None, NONE)
            .await?)
    }

    /// Company profile.
    pub async fn profile(&self, symbol: &str) -> Result<Profile, Error> {
        Ok(self
            .inner
            .get_profile(Some(symbol), NONE, NONE, NONE, NONE, NONE, NONE)
            .await?)
    }

    /// Valuation, financial and dividend statistics.
    pub async fn statistics(&self, symbol: &str) -> Result<Statistics, Error> {
        Ok(self
            .inner
            .get_statistics(Some(symbol), NONE, NONE, NONE, NONE, NONE, NONE)
            .await?)
    }

    /// Analyst price targets.
    pub async fn price_target(&self, symbol: &str) -> Result<PriceTarget, Error> {
        Ok(self
            .inner
            .get_price_target(Some(symbol), NONE, NONE, NONE, NONE, NONE, NONE)
            .await?)
    }

    /// Past and upcoming earnings dates with EPS estimates.
    pub async fn earnings(&self, symbol: &str) -> Result<Earnings, Error> {
        Ok(self
            .inner
            .get_earnings(
                Some(symbol),
                NONE,
                NONE,
                NONE,
                NONE,
                NONE,
                NONE,
                None,
                None,
                None,
                None,
                NONE,
                NONE,
                NONE,
                None,
            )
            .await?)
    }

    /// Instruments matching a symbol or name fragment.
    pub async fn symbol_search(&self, query: &str) -> Result<SymbolSearch, Error> {
        Ok(self.inner.get_symbol_search(query, None, None).await?)
    }

    /// Credits used in the current minute and day against the plan limits.
    pub async fn api_usage(&self) -> Result<ApiUsage, Error> {
        Ok(self.inner.get_api_usage(None, NONE, NONE).await?)
    }

    async fn get_json(&self, path: &str, query: &[(&str, String)]) -> Result<Value, Error> {
        let response = self
            .http
            .get(format!("{}{path}", self.base_url))
            .query(query)
            .header(reqwest::header::AUTHORIZATION, &self.authorization)
            .header(reqwest::header::ACCEPT, "application/json")
            .send()
            .await
            .map_err(HttpError::Network)?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = read_bounded(response, self.max_response_body_bytes).await?;
        let body = String::from_utf8_lossy(&body).into_owned();
        match serde_json::from_str::<Value>(&body) {
            Ok(value)
                if (200..300).contains(&status) && ApiMessage::from_value(&value).is_none() =>
            {
                Ok(value)
            }
            Ok(_) => Err(Error::from_response(status, body, &headers, None)),
            Err(error) => Err(Error::from_response(
                status,
                body,
                &headers,
                Some(error.to_string()),
            )),
        }
    }
}

async fn read_bounded(response: reqwest::Response, limit: usize) -> Result<Vec<u8>, HttpError> {
    use futures_util::StreamExt;
    let mut body = Vec::new();
    let mut stream = std::pin::pin!(response.bytes_stream());
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(HttpError::Network)?;
        if body.len() + chunk.len() > limit {
            return Err(HttpError::ResponseTooLarge { limit });
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
