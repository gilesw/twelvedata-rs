//! HTTP contract tests using fixtures built from the specification's property examples.
use serde_json::{Value, json};
use twelvedata_api::{Client, Error, Interval, Order, TimeSeriesQuery};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const KEY: &str = "fixture-key";

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn fixture_json(name: &str) -> Value {
    serde_json::from_str(&fixture(name)).unwrap()
}

fn client(server: &MockServer) -> Client {
    Client::new(KEY).with_base_url(format!("{}/", server.uri()))
}

async fn mount(server: &MockServer, route: &str, symbol: &str, name: &str) {
    Mock::given(method("GET"))
        .and(path(route))
        .and(query_param("symbol", symbol))
        .and(header("authorization", &format!("apikey {KEY}")))
        .and(header("accept", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture(name)))
        .expect(1)
        .mount(server)
        .await;
}

fn error_body(code: u16, message: &str) -> Value {
    json!({"code": code, "message": message, "status": "error"})
}

#[tokio::test]
async fn quote_uses_documented_endpoint_auth_and_fields() {
    let server = MockServer::start().await;
    mount(&server, "/quote", "AAPL", "quote").await;
    let quote = client(&server).quote("AAPL").await.unwrap();
    assert_eq!(quote.symbol, "AAPL");
    assert_eq!(quote.close, "148.85001");
    assert_eq!(quote.percent_change, "-0.16097");
    assert_eq!(quote.currency.as_deref(), Some("USD"));
    assert_eq!(quote.volume.as_deref(), Some("67903927"));
    assert_eq!(quote.timestamp, 1631772000);
    assert!(!quote.is_market_open);
    assert_eq!(quote.fifty_two_week.high.as_deref(), Some("157.25999"));
    assert_eq!(quote.fifty_two_week.low.as_deref(), Some("103.10000"));
    let requests = server.received_requests().await.unwrap();
    assert!(
        !requests[0].url.query().unwrap().contains(KEY),
        "API key must not be in URL"
    );
}

#[tokio::test]
async fn batch_quotes_are_keyed_by_symbol_with_per_symbol_errors() {
    let server = MockServer::start().await;
    let mut msft = fixture_json("quote");
    msft["symbol"] = json!("MSFT");
    msft["close"] = json!("300.5");
    Mock::given(method("GET"))
        .and(path("/quote"))
        .and(query_param("symbol", "AAPL,MSFT,ZZZZ"))
        .and(header("authorization", &format!("apikey {KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "AAPL": fixture_json("quote"),
            "MSFT": msft,
            "ZZZZ": error_body(404, "**symbol** not found: ZZZZ"),
        })))
        .expect(1)
        .mount(&server)
        .await;
    let quotes = client(&server)
        .quotes(&["AAPL", "MSFT", "ZZZZ"])
        .await
        .unwrap();
    assert_eq!(quotes.len(), 3);
    assert_eq!(quotes["AAPL"].as_ref().unwrap().close, "148.85001");
    assert_eq!(quotes["MSFT"].as_ref().unwrap().close, "300.5");
    let missing = quotes["ZZZZ"].as_ref().unwrap_err();
    assert_eq!(missing.code, 404);
    assert!(missing.message.contains("ZZZZ"));
    let requests = server.received_requests().await.unwrap();
    assert!(!requests[0].url.query().unwrap().contains(KEY));
}

#[tokio::test]
async fn single_symbol_batch_uses_the_single_quote_shape() {
    let server = MockServer::start().await;
    mount(&server, "/quote", "AAPL", "quote").await;
    let quotes = client(&server).quotes(&["AAPL"]).await.unwrap();
    assert_eq!(quotes["AAPL"].as_ref().unwrap().symbol, "AAPL");
}

#[tokio::test]
async fn batch_quotes_report_whole_request_failures() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/quote"))
        .respond_with(ResponseTemplate::new(200).set_body_json(error_body(401, "bad key")))
        .mount(&server)
        .await;
    let error = client(&server).quotes(&["AAPL", "MSFT"]).await.unwrap_err();
    assert!(error.is_unauthorized());
    let none: [&str; 0] = [];
    assert!(
        client(&server)
            .quotes(&none)
            .await
            .is_err_and(|e| matches!(e, Error::InvalidRequest(_)))
    );
}

#[tokio::test]
async fn time_series_sends_query_and_decodes_bars() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/time_series"))
        .and(query_param("symbol", "AAPL"))
        .and(query_param("interval", "1day"))
        .and(query_param("outputsize", "250"))
        .and(query_param("start_date", "2021-09-01"))
        .and(query_param("order", "asc"))
        .and(query_param("timezone", "Exchange"))
        .and(header("authorization", &format!("apikey {KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("time_series")))
        .expect(1)
        .mount(&server)
        .await;
    let query = TimeSeriesQuery {
        outputsize: Some(250),
        start_date: Some("2021-09-01".to_owned()),
        order: Some(Order::Asc),
        timezone: Some("Exchange".to_owned()),
        ..TimeSeriesQuery::new("AAPL", Interval::Variant1day)
    };
    let series = client(&server).time_series(&query).await.unwrap();
    assert_eq!(series.status, "ok");
    assert_eq!(series.meta.currency.as_deref(), Some("USD"));
    assert_eq!(
        series.meta.exchange_timezone.as_deref(),
        Some("America/New_York")
    );
    assert_eq!(series.values.len(), 1);
    assert_eq!(series.values[0].datetime, "2021-09-16 15:59:00");
    assert_eq!(series.values[0].close, "148.85001");
    assert_eq!(series.values[0].volume.as_deref(), Some("624277"));
}

#[tokio::test]
async fn time_series_rejects_blank_symbol_before_request() {
    let server = MockServer::start().await;
    let error = client(&server)
        .time_series(&TimeSeriesQuery::new(" ", Interval::Variant1day))
        .await
        .unwrap_err();
    assert!(matches!(error, Error::InvalidRequest(_)));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn price_eod_and_exchange_rate_decode() {
    let server = MockServer::start().await;
    mount(&server, "/price", "AAPL", "price").await;
    mount(&server, "/eod", "AAPL", "eod").await;
    mount(&server, "/exchange_rate", "USD/JPY", "exchange_rate").await;
    let client = client(&server);
    assert_eq!(client.price("AAPL").await.unwrap().price, "200.99001");
    let eod = client.eod("AAPL").await.unwrap();
    assert_eq!(eod.close, "148.79");
    assert_eq!(eod.datetime, "2021-09-16");
    let rate = client.exchange_rate("USD/JPY").await.unwrap();
    assert_eq!(rate.rate, 105.12);
    assert_eq!(rate.timestamp, 1602714051);
}

#[tokio::test]
async fn fundamentals_decode_nested_sections() {
    let server = MockServer::start().await;
    mount(&server, "/profile", "AAPL", "profile").await;
    mount(&server, "/statistics", "AAPL", "statistics").await;
    mount(&server, "/price_target", "AAPL", "price_target").await;
    mount(&server, "/earnings", "AAPL", "earnings").await;
    let client = client(&server);

    let profile = client.profile("AAPL").await.unwrap();
    assert_eq!(profile.name, "Apple Inc");
    assert_eq!(profile.sector.as_deref(), Some("Technology"));

    let statistics = client.statistics("AAPL").await.unwrap();
    let valuations = statistics.statistics.valuations_metrics.unwrap();
    assert_eq!(valuations.market_capitalization, Some(2546807865344.0));
    assert_eq!(valuations.trailing_pe, Some(30.162493));
    let financials = statistics.statistics.financials.unwrap();
    assert!(financials.return_on_equity_ttm.is_some());
    assert!(
        statistics
            .statistics
            .dividends_and_splits
            .unwrap()
            .payout_ratio
            .is_some()
    );

    let target = client.price_target("AAPL").await.unwrap();
    assert_eq!(target.price_target.average, Some(184.01));
    assert_eq!(target.price_target.high, Some(220.0));
    assert_eq!(target.price_target.currency, "USD");

    let earnings = client.earnings("AAPL").await.unwrap();
    assert_eq!(earnings.earnings[0].date, "2020-04-30");
    assert_eq!(earnings.earnings[0].eps_estimate, Some(2.09));
}

#[tokio::test]
async fn symbol_search_and_api_usage_decode() {
    let server = MockServer::start().await;
    mount(&server, "/symbol_search", "Alcoa", "symbol_search").await;
    Mock::given(method("GET"))
        .and(path("/api_usage"))
        .and(header("authorization", &format!("apikey {KEY}")))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("api_usage")))
        .expect(1)
        .mount(&server)
        .await;
    let client = client(&server);
    let found = client.symbol_search("Alcoa").await.unwrap();
    assert_eq!(found.data[0].symbol, "AA");
    assert_eq!(found.data[0].mic_code, "XNYS");
    let usage = client.api_usage().await.unwrap();
    assert_eq!(usage.current_usage, 4003);
    assert_eq!(usage.plan_limit, 20000);
}

#[tokio::test]
async fn error_reported_inside_http_200_is_an_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/quote"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(error_body(429, "You have run out of API credits")),
        )
        .mount(&server)
        .await;
    let error = client(&server).quote("AAPL").await.unwrap_err();
    assert!(error.is_rate_limited());
    assert!(!error.is_unauthorized());
    match &error {
        Error::Api {
            status,
            code,
            message,
            ..
        } => {
            assert_eq!(*status, 200);
            assert_eq!(*code, Some(429));
            assert_eq!(message.as_deref(), Some("You have run out of API credits"));
        }
        other => panic!("unexpected error: {other}"),
    }
    assert!(error.to_string().contains("code 429"));
}

#[tokio::test]
async fn http_errors_keep_status_message_body_and_retry_after() {
    for status in [401, 403, 404, 429] {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(status)
                    .insert_header("retry-after", "60")
                    .set_body_json(error_body(status, "rejected")),
            )
            .mount(&server)
            .await;
        let error = client(&server).quote("AAPL").await.unwrap_err();
        assert_eq!(error.is_unauthorized(), matches!(status, 401 | 403));
        assert_eq!(error.is_rate_limited(), status == 429);
        assert_eq!(error.is_not_found(), status == 404);
        match error {
            Error::Api {
                status: actual,
                code,
                message,
                body,
                retry_after,
            } => {
                assert_eq!(actual, status);
                assert_eq!(code, Some(i64::from(status)));
                assert_eq!(message.as_deref(), Some("rejected"));
                assert!(body.contains("rejected"));
                assert_eq!(retry_after.as_deref(), Some("60"));
            }
            error => panic!("unexpected error: {error}"),
        }
    }
}

#[tokio::test]
async fn non_json_http_error_keeps_body_without_code() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
        .mount(&server)
        .await;
    match client(&server).quote("AAPL").await.unwrap_err() {
        Error::Api {
            status, code, body, ..
        } => {
            assert_eq!(status, 502);
            assert_eq!(code, None);
            assert_eq!(body, "bad gateway");
        }
        error => panic!("unexpected error: {error}"),
    }
}

#[tokio::test]
async fn malformed_success_is_distinct_from_http_failure() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"symbol":"AAPL"}"#))
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server).quote("AAPL").await,
        Err(Error::Decode(_))
    ));
    assert!(matches!(
        client(&server).quotes(&["AAPL", "MSFT"]).await,
        Err(Error::Decode(_))
    ));
}

#[tokio::test]
async fn raw_client_exposes_every_query_parameter() {
    let server = MockServer::start().await;
    Mock::given(path("/quote"))
        .and(header("authorization", &format!("apikey {KEY}")))
        .and(query_param("symbol", "VOD"))
        .and(query_param("mic_code", "XLON"))
        .and(query_param("prepost", "true"))
        .and(query_param("dp", "4"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("quote")))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .raw()
        .get_quote(
            Some("VOD"),
            None::<&str>,
            None::<&str>,
            None::<&str>,
            None,
            None::<&str>,
            Some("XLON"),
            None::<&str>,
            None,
            None,
            None,
            None::<&str>,
            Some(true),
            None,
            None,
            Some(4),
            None::<&str>,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn response_size_limit_is_enforced_on_both_transports() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/quote"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture("quote")))
        .mount(&server)
        .await;
    let client = client(&server).with_max_response_body_bytes(16);
    assert!(matches!(
        client.quote("AAPL").await,
        Err(Error::Transport(
            twelvedata_api::generated::client::HttpError::ResponseTooLarge { limit: 16 }
        ))
    ));
    assert!(matches!(
        client.quotes(&["AAPL", "MSFT"]).await,
        Err(Error::Transport(
            twelvedata_api::generated::client::HttpError::ResponseTooLarge { limit: 16 }
        ))
    ));
}

#[test]
fn client_debug_omits_api_key() {
    assert!(!format!("{:?}", Client::new(KEY)).contains(KEY));
}
