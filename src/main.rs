use anyhow::{Context, Result};
use nostr_rust_news::{client::RedditClient, nostr::post_nostr};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<()> {
    let dry_run = env::args().any(|arg| arg == "--dry-run");
    let client = RedditClient::new()?;
    let posts = client.fetch_rust_posts().await?;

    let nsec = env::var("NOSTR_NSEC").context("NOSTR_NSEC is required")?;
    let relays: Vec<String> = env::var("NOSTR_RELAYS")
        .context("NOSTR_RELAYS is required (comma-separated relay URLs)")?
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if relays.is_empty() {
        anyhow::bail!("NOSTR_RELAYS must contain at least one relay URL");
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64();
    let cutoff = now - 3600.0;

    for post in posts.into_iter().filter(|p| p.created_utc >= cutoff) {
        let permalink = format!("https://www.reddit.com{}", post.permalink);
        let text = format!(
            "{}\n{}\n\nr/rust by u/{} • score {} • comments {}\n{}",
            post.title, post.url, post.author, post.score, post.num_comments, permalink
        );

        if dry_run {
            println!("dry-run: {}", post.title);
            println!("{}", text);
        } else {
            let event_id = post_nostr(&nsec, &relays, &text).await?;
            println!("posted: {} ({})", post.title, event_id);
        }
    }

    Ok(())
}
