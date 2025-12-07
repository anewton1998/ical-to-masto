use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub instance: String,
    #[serde(default = "default_token_file")]
    pub token_file: String,
    pub webcal: String,
}

fn default_token_file() -> String {
    "token.json".to_string()
}

pub fn load_config(config_path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&content)?;
    println!("Configuration loaded from: {}", config_path);
    println!("Instance: {}", config.instance);
    Ok(config)
}

pub fn load_token(config: &Config) -> Result<mastodon_async::Data, Box<dyn std::error::Error>> {
    let token_file_path = &config.token_file;

    if !std::path::Path::new(token_file_path).exists() {
        return Err("No authentication token found. Please run 'login' command first.".into());
    }

    let content = std::fs::read_to_string(token_file_path)?;
    let data: mastodon_async::Data = serde_json::from_str(&content)?;
    Ok(data)
}

pub fn save_token(
    config: &Config,
    token_data: &mastodon_async::Data,
) -> Result<(), Box<dyn std::error::Error>> {
    let token_file_path = &config.token_file;

    // Create parent directory if it doesn't exist
    if let Some(parent) = std::path::Path::new(token_file_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(token_data)?;
    std::fs::write(token_file_path, json)?;

    println!("Authentication token saved to: {}", token_file_path);
    Ok(())
}
