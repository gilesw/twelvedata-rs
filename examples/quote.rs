use twelvedata_api::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::io::read_to_string(std::io::stdin())?;
    let symbol = std::env::args().nth(1).unwrap_or_else(|| "AAPL".to_owned());
    let client = Client::new(api_key.trim());
    let quote = client.quote(&symbol).await?;
    println!("{}", serde_json::to_string_pretty(&quote)?);
    Ok(())
}
