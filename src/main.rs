use anyhow::{Context, Result};
use nostr_rust_news::{client::RedditClient, github::GitHubClient, nostr::post_nostr};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let reddit_only = args.iter().any(|arg| arg == "--reddit");
    let github_only = args.iter().any(|arg| arg == "--github");

    let fetch_reddit = !github_only;
    let fetch_github = !reddit_only;

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

    // Fetch and post Reddit content
    if fetch_reddit {
        let client = RedditClient::new()?;
        let posts = client.fetch_rust_posts().await?;

        for post in posts.into_iter().filter(|p| p.created_utc >= cutoff) {
            let permalink = format!("https://www.reddit.com{}", post.permalink);
            let text = format!(
                "{}\n{}\n\nr/rust by u/{} • score {} • comments {}\n{}",
                post.title, post.url, post.author, post.score, post.num_comments, permalink
            );

            if dry_run {
                println!("dry-run [Reddit]: {}", post.title);
                println!("{}\n", text);
            } else {
                let event_id = post_nostr(&nsec, &relays, &text).await?;
                println!("posted [Reddit]: {} ({})", post.title, event_id);
            }
        }
    }

    // Fetch and post GitHub trending content
    if fetch_github {
        let client = GitHubClient::new()?;
        let repos = client.fetch_trending("rust", "daily").await?;

        for repo in repos.into_iter() {
            // Parse stars_today to check if >= 100
            let stars_today_num = repo.stars_today
                .split_whitespace()
                .next()
                .and_then(|s| s.replace(",", "").parse::<i32>().ok())
                .unwrap_or(0);

            // Only post if it got 100+ stars today
            if stars_today_num < 100 {
                continue;
            }

            let text = format!(
                "🔥 Trending Rust Repository\n\n{}/{}\n{}\n\n{}\n\n⭐ {} stars | 🍴 {} forks | 📈 {} stars today",
                repo.author,
                repo.name,
                repo.url,
                repo.description,
                repo.stars,
                repo.forks,
                repo.stars_today
            );

            if dry_run {
                println!("dry-run [GitHub]: {}/{} ({} stars today)", repo.author, repo.name, stars_today_num);
                println!("{}\n", text);
            } else {
                let event_id = post_nostr(&nsec, &relays, &text).await?;
                println!("posted [GitHub]: {}/{} ({} stars today) ({})", repo.author, repo.name, stars_today_num, event_id);
            }
        }
    }

    Ok(())
}
