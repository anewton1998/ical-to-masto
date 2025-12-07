use clap::{Parser, Subcommand};
use std::str::FromStr;
mod config;
use ical_to_masto::ical::IcalCalendar;

#[derive(Parser)]
#[command(name = "ical-to-masto")]
#[command(about = "A tool to sync iCal events to Mastodon")]
struct Cli {
    #[arg(
        short = 'c',
        long,
        help = "Path to TOML configuration file",
        default_value = "bot.toml"
    )]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Register an application with a Mastodon instance")]
    Register {
        #[arg(short, long, default_value = "ical-to-masto")]
        client_name: String,
        #[arg(short, long)]
        redirect_uri: Option<String>,
        #[arg(short, long, default_values = ["write:statuses"])]
        scopes: Vec<String>,
        #[arg(short, long)]
        website: Option<String>,
    },
    #[command(about = "Post the next meeting from iCal to Mastodon")]
    PostNext {},
    #[command(about = "Post all upcoming meetings from iCal to Mastodon")]
    PostAll {},
    #[command(about = "Post a status to Mastodon")]
    PostStatus {
        #[arg(help = "Status text to post")]
        status: String,
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
        Commands::Register {
            client_name,
            redirect_uri,
            scopes,
            website,
        } => {
            if let Err(e) = register(
                &config,
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
        Commands::PostNext {} => {
            if let Err(e) = post_next_meeting(&config).await {
                eprintln!("Error posting next meeting: {}", e);
                std::process::exit(1);
            }
        }
        Commands::PostAll {} => {
            if let Err(e) = post_all_upcoming_meetings(&config).await {
                eprintln!("Error posting all upcoming meetings: {}", e);
                std::process::exit(1);
            }
        }
        Commands::PostStatus { status } => {
            if let Err(e) = post_status(&config, &status).await {
                eprintln!("Error posting status: {}", e);
                std::process::exit(1);
            }
        }
    }
}

async fn register(
    config: &config::Config,
    client_name: &str,
    redirect_uri: Option<&str>,
    scopes: Option<&str>,
    website: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::Registration;

    let mut registration = Registration::new(&config.instance);
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

            println!("\nAfter authorizing, paste the authorization code here:");
            let mut code = String::new();
            std::io::stdin().read_line(&mut code)?;
            let code = code.trim();

            match app.complete(code).await {
                Ok(mastodon) => {
                    println!("Authentication successful!");

                    // Save the authenticated data
                    let token_data = mastodon.data.clone();
                    config::save_token(config, &token_data)?;
                }
                Err(e) => {
                    println!("Error completing authentication: {}", e);
                    println!("You may need to use the 'login' command with client credentials.");
                }
            }
        }
        Err(e) => {
            println!("Error generating authorize URL: {}", e);
            println!("Use the 'login' command with the client credentials to authenticate.");
        }
    }

    Ok(())
}

async fn post_next_meeting(config: &config::Config) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::{Mastodon, NewStatus};

    let data = config::load_token(config)?;
    let mastodon = Mastodon::from(data);

    // Load calendar from webcal URL
    let calendar = IcalCalendar::from_url(&config.webcal).await?;

    // Get current time in iCal format
    let current_time = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    // Get upcoming events (limit to 1 for next meeting)
    let upcoming_events = calendar.get_upcoming_events_limited(&current_time, Some(1));

    let status = if let Some(event) = upcoming_events.first() {
        // Format meeting details
        let summary = event.summary.as_deref().unwrap_or("Meeting");
        let location = event.location.as_deref().unwrap_or("Location TBD");
        let start_time = event
            .start_time_formatted()
            .unwrap_or("Time TBD".to_string());
        let event_url = event.url.as_deref();

        if let Some(url) = event_url {
            format!(
                "📅 Next Meeting: {}\n📍 {}\n🕒 {}\n🔗 {}",
                summary, location, start_time, url
            )
        } else {
            format!(
                "📅 Next Meeting: {}\n📍 {}\n🕒 {}",
                summary, location, start_time
            )
        }
    } else {
        "📅 No upcoming meetings found".to_string()
    };

    let new_status = NewStatus {
        status: Some(status),
        ..Default::default()
    };

    let posted_status = mastodon.new_status(new_status).await?;

    println!("Next meeting posted successfully!");
    println!("ID: {}", posted_status.id);
    if let Some(url) = posted_status.url {
        println!("URL: {}", url);
    }

    Ok(())
}

async fn post_all_upcoming_meetings(
    config: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    use mastodon_async::{Mastodon, NewStatus};

    let data = config::load_token(config)?;
    let mastodon = Mastodon::from(data);

    // Load calendar from webcal URL
    let calendar = IcalCalendar::from_url(&config.webcal).await?;

    // Get current time in iCal format
    let current_time = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

    // Get all upcoming events (no limit)
    let upcoming_events = calendar.get_upcoming_events(&current_time);

    let status = if upcoming_events.is_empty() {
        "📅 No upcoming meetings found".to_string()
    } else {
        let mut meeting_list = String::new();

        for (i, event) in upcoming_events.iter().enumerate() {
            // Format meeting details
            let summary = event.summary.as_deref().unwrap_or("Meeting");
            let location = event.location.as_deref().unwrap_or("Location TBD");
            let start_time = event
                .start_time_formatted()
                .unwrap_or("Time TBD".to_string());

            if i > 0 {
                meeting_list.push_str("\n\n");
            }

            let event_url = event.url.as_deref();
            let meeting_line = if let Some(url) = event_url {
                format!(
                    "📅 {}\n📍 {}\n🕒 {}\n🔗 {}\n",
                    summary, location, start_time, url
                )
            } else {
                format!("📅 {}\n📍 {}\n🕒 {}\n", summary, location, start_time)
            };

            meeting_list.push_str(&meeting_line);
        }

        format!(
            "Upcoming Meetings ({}):\n\n{}",
            upcoming_events.len(),
            meeting_list
        )
    };

    let new_status = NewStatus {
        status: Some(status),
        ..Default::default()
    };

    let posted_status = mastodon.new_status(new_status).await?;

    println!("Posted upcoming meetings status: {}", posted_status.id);
    if let Some(url) = posted_status.url {
        println!("URL: {}", url);
    }

    Ok(())
}

async fn post_status(
    config: &config::Config,
    status: &str,
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
