use clap::{Parser, Subcommand};
use std::str::FromStr;
mod config;

#[derive(Parser)]
#[command(name = "ical-to-masto")]
#[command(about = "A tool to sync iCal events to Mastodon")]
struct Cli {
    #[arg(short = 'c', long, help = "Path to TOML configuration file", default_value = "bot.toml")]
    config: Option<String>,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Register an application with a Mastodon instance")]
    RegisterApp {
        #[arg(short, long, default_value = "ical-to-masto")]
        client_name: String,
        #[arg(short, long)]
        redirect_uri: Option<String>,
        #[arg(short, long, default_values = ["write:statuses"])]
        scopes: Vec<String>,
        #[arg(short, long)]
        website: Option<String>,
    },
    #[command(about = "Authenticate with a Mastodon instance")]
    Login {
        #[arg(long)]
        client_id: String,
        #[arg(long)]
        client_secret: String,
        #[arg(short, long)]
        redirect_uri: Option<String>,
    },
    #[command(about = "Post a status to Mastodon")]
    Post {
        #[arg(short, long)]
        status: String,
        #[arg(long)]
        visibility: Option<String>,
        #[arg(long)]
        sensitive: Option<bool>,
        #[arg(long)]
        spoiler_text: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        in_reply_to_id: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    // Load configuration file (will use default "bot.toml" if not specified)
    let config = match config::load_config(&cli.config.as_ref().unwrap()) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::RegisterApp {
            client_name,
            redirect_uri,
            scopes,
            website,
        } => {
            if let Err(e) = register_app(
                &config.instance,
                &client_name,
                redirect_uri.as_deref(),
                Some(&scopes.join(" ")),
                website.as_deref(),
            )
            .await
            {
                eprintln!("Error registering app: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Login {
            client_id,
            client_secret,
            redirect_uri,
        } => {
            if let Err(e) = login(
                &config,
                &client_id,
                &client_secret,
                redirect_uri.as_deref(),
            )
            .await
            {
                eprintln!("Error during login: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Post {
            status,
            visibility,
            sensitive,
            spoiler_text,
            language,
            in_reply_to_id,
        } => {
            if let Err(e) = post_status(
                &config,
                &status,
                visibility.as_deref(),
                sensitive,
                spoiler_text.as_deref(),
                language.as_deref(),
                in_reply_to_id.as_deref(),
            )
            .await
            {
                eprintln!("Error posting status: {}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn register_app(
    instance: &str,
    client_name: &str,
    redirect_uri: Option<&str>,
    scopes: Option<&str>,
    website: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::Registration;

    let mut registration = Registration::new(instance);
    registration.client_name(client_name);

    if let Some(uri) = redirect_uri {
        registration.redirect_uris(uri);
    }

    if let Some(scope_str) = scopes {
        use mastodon_async::prelude::Scopes;
        let scopes = Scopes::from_str(scope_str)?;
        registration.scopes(scopes);
    }

    if let Some(website_url) = website {
        registration.website(website_url);
    }

    let app = registration.build().await?;

    println!("Application registered successfully!");
    
    match app.authorize_url() {
        Ok(authorize_url) => {
            println!("\nPlease open this URL in your browser to authorize the application:");
            println!("{}", authorize_url);
            println!("\nAfter authorizing, you'll need to use the 'login' command with the client credentials.");
        }
        Err(e) => {
            println!("Error generating authorize URL: {}", e);
            println!("Use the 'login' command with the client credentials to authenticate.");
        }
    }

    Ok(())
}

async fn login(
    config: &config::Config,
    client_id: &str,
    client_secret: &str,
    redirect_uri: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::Data;
    use std::borrow::Cow;

    let redirect = redirect_uri.unwrap_or("urn:ietf:wg:oauth:2.0:oob");

    println!("Please open this URL in your browser to authorize the application:");
    println!(
        "{}?client_id={}&response_type=code&redirect_uri={}&scope=read",
        format!("{}/oauth/authorize", config.instance),
        client_id,
        redirect
    );

    println!("\nAfter authorizing, paste the authorization code here:");
    let mut code = String::new();
    std::io::stdin().read_line(&mut code)?;
    let code = code.trim();

    // Create a simple token request
    let client = reqwest::Client::new();
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect),
    ];

    let response = client
        .post(&format!("{}/oauth/token", config.instance))
        .form(&params)
        .send()
        .await?;

    let text = response.text().await?;
    let token_response: serde_json::Value = serde_json::from_str(&text)?;

    let access_token = token_response["access_token"]
        .as_str()
        .ok_or("No access token in response")?
        .to_string();

    println!("Login successful!");

    // Save the token to a file for future use
    let token_data = Data {
        base: Cow::Owned(config.instance.to_string()),
        client_id: Cow::Owned(client_id.to_string()),
        client_secret: Cow::Owned(client_secret.to_string()),
        token: Cow::Owned(access_token),
        ..Default::default()
    };

    config::save_token(config, &token_data)?;

    Ok(())
}





async fn post_status(
    config: &config::Config,
    status: &str,
    _visibility: Option<&str>,
    _sensitive: Option<bool>,
    _spoiler_text: Option<&str>,
    _language: Option<&str>,
    _in_reply_to_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::{Mastodon, NewStatus};

    let data = config::load_token(config)?;
    let mastodon = Mastodon::from(data);

    let new_status = NewStatus {
        status: Some(status.to_string()),
        ..Default::default()
    };

    let posted_status = mastodon.new_status(new_status).await?;

    println!("Status posted successfully!");
    println!("ID: {}", posted_status.id);
    if let Some(url) = posted_status.url {
        println!("URL: {}", url);
    }

    Ok(())
}
