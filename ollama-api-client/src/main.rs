use ollama_api_client::OllamaControlsApiClient;

fn main() {
    let client = OllamaControlsApiClient::from_env();
    match client.health() {
        Ok(h) => println!("health: {}", h.ok),
        Err(e) => eprintln!("health failed: {e}"),
    }
}
