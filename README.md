# ical-to-masto

A Rust tool to post iCal calendar events to Mastodon.
This tool can post upcoming meetings and events from an iCal calendar to your Mastodon account.

The fediverse bot was originally developed for the [Northern Virginia Linux Users Group](https://novalug.org).

## Features

- Fetch iCal calendars from web URLs
- Post the next upcoming meeting to Mastodon
- Post all upcoming meetings to Mastodon
- Post custom status updates
- OAuth2 authentication with Mastodon instances
- Configurable via TOML files

This is known to work with Pleroma/Akkoma instances as well.

## Installation

### From source

```bash
git clone <repository-url>
cd ical-to-masto
cargo build --release
```

The binary will be available at `target/release/ical-to-masto`.

## Configuration

Create a TOML configuration file (default: `bot.toml`):

```toml
instance = "https://mastodon.social"
token_file = "token.json"
webcal = "https://example.com/calendar.ics"
```

- `instance`: Your Mastodon instance URL
- `token_file`: Path to store authentication token (default: `token.json`)
- `webcal`: URL to the iCal calendar file

## Usage

### 1. Register the application

First, register the application with your Mastodon instance:

```bash
ical-to-masto register -c bot.toml
```

This will:
- Register the application with your Mastodon instance
- Provide an authorization URL
- Prompt you to paste the authorization code
- Save the authentication token to the specified token file

### 2. Post meetings

Once authenticated, you can post meetings:

```bash
# Post the next upcoming meeting
ical-to-masto post-next -c bot.toml

# Post all upcoming meetings
ical-to-masto post-all -c bot.toml

# Post a custom status
ical-to-masto post-status "Hello from ical-to-masto!" -c bot.toml
```

## Status Format

The tool formats meeting posts with emojis and includes:

- 📅 Meeting title/summary
- 📍 Location (if available)
- 🕒 Start time (formatted as readable date/time)
- 🔗 Event URL (if available)

Example output:
```
📅 Next Meeting: Team Standup
📍 Conference Room A
🕒 Mon, Dec 07, 2025 at 10:00 AM
🔗 https://example.com/meeting-link
```

## License

This project is dual-licensed under the Apache License 2.0 and MIT License. See LICENSE.md for details.
