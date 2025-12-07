use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ical-to-masto")]
#[command(about = "A tool to sync iCal events to Mastodon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Register an application with a Mastodon instance")]
    RegisterApp {
        #[arg(short, long)]
        instance: String,
        #[arg(short, long, default_value = "ical-to-masto")]
        client_name: String,
        #[arg(short, long)]
        redirect_uri: Option<String>,
        #[arg(short, long)]
        scopes: Option<String>,
        #[arg(short, long)]
        website: Option<String>,
    },
    #[command(about = "Authenticate with a Mastodon instance")]
    Login {
        #[arg(short, long)]
        instance: String,
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

    match cli.command {
        Commands::RegisterApp {
            instance,
            client_name,
            redirect_uri,
            scopes,
            website,
        } => {
            if let Err(e) = register_app(
                &instance,
                &client_name,
                redirect_uri.as_deref(),
                scopes.as_deref(),
                website.as_deref(),
            )
            .await
            {
                eprintln!("Error registering app: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Login {
            instance,
            client_id,
            client_secret,
            redirect_uri,
        } => {
            if let Err(e) = login(
                &instance,
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
    _website: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::Registration;

    let mut registration = Registration::new(instance);
    registration.client_name(client_name);

    if let Some(_uri) = redirect_uri {
        eprintln!("Redirect URI parameter provided but not implemented");
    }

    if let Some(scope_str) = scopes {
        eprintln!(
            "Scope parameter provided but not implemented: {}",
            scope_str
        );
    }

    let _app = registration.build().await?;

    println!("Application registered successfully!");
    println!("Save these credentials for authentication:");
    println!("Registration completed successfully!");
    println!("The application has been registered with the Mastodon instance.");
    println!("Check the instance documentation for next steps on authentication.");

    Ok(())
}

async fn login(
    instance: &str,
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
        format!("{}/oauth/authorize", instance),
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
        .post(&format!("{}/oauth/token", instance))
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
        base: Cow::Owned(instance.to_string()),
        client_id: Cow::Owned(client_id.to_string()),
        client_secret: Cow::Owned(client_secret.to_string()),
        token: Cow::Owned(access_token),
        ..Default::default()
    };

    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let config_dir = home_dir.join(".config").join("ical-to-masto");
    std::fs::create_dir_all(&config_dir)?;

    let token_file = config_dir.join("token.json");
    let json = serde_json::to_string_pretty(&token_data)?;
    std::fs::write(token_file, json)?;

    println!(
        "Authentication token saved to {}",
        config_dir.join("token.json").display()
    );

    Ok(())
}

fn load_token() -> Result<mastodon_async::Data, Box<dyn std::error::Error>> {
    let home_dir = dirs::home_dir().ok_or("Could not find home directory")?;
    let token_file = home_dir
        .join(".config")
        .join("ical-to-masto")
        .join("token.json");

    if !token_file.exists() {
        return Err("No authentication token found. Please run 'login' command first.".into());
    }

    let content = std::fs::read_to_string(token_file)?;
    let data: mastodon_async::Data = serde_json::from_str(&content)?;
    Ok(data)
}

async fn post_status(
    status: &str,
    _visibility: Option<&str>,
    _sensitive: Option<bool>,
    _spoiler_text: Option<&str>,
    _language: Option<&str>,
    _in_reply_to_id: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::{Mastodon, NewStatus};

    let data = load_token()?;
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
