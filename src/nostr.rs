use anyhow::Result;
use nostr_sdk::prelude::*;
use std::time::Duration;

/// Every note is tagged with these, so it shows up in #rust feeds.
pub const HASHTAGS: &[&str] = &["rust", "rustlang"];

/// A text note with the hashtags as `t` tags and at the end of the text (some clients only show the latter).
pub fn note(text: &str) -> EventBuilder {
    let shown: Vec<String> = HASHTAGS.iter().map(|t| format!("#{t}")).collect();
    EventBuilder::text_note(format!("{text}\n\n{}", shown.join(" ")))
        .tags(HASHTAGS.iter().map(|t| Tag::hashtag(*t)))
}

async fn send(nsec: &str, relays: &[String], builder: EventBuilder) -> Result<String> {
    let client = Client::new(Keys::parse(nsec)?);
    for r in relays {
        if let Ok(url) = RelayUrl::parse(r) {
            let _ = client.add_relay(url).await;
        }
    }
    client.connect().await;
    let output = client.send_event_builder(builder).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    client.disconnect().await;
    Ok(output.id().to_bech32()?)
}

pub async fn post_nostr(nsec: &str, relays: &[String], text: &str) -> Result<String> {
    send(nsec, relays, note(text)).await
}

/// Publishes a NIP-65 relay list (kind 10002) naming `relays` for reading and writing,
/// so clients know where to find the account's notes.
pub async fn publish_relay_list(nsec: &str, relays: &[String]) -> Result<String> {
    let list = relays.iter().filter_map(|r| RelayUrl::parse(r).ok()).map(|url| (url, None));
    send(nsec, relays, EventBuilder::relay_list(list)).await
}
