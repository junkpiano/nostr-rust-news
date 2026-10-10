use anyhow::{Context, Result};
use nostr_rust_news::{
    blog,
    client::RedditClient,
    github::GitHubClient,
    nostr::{post_nostr, publish_relay_list},
};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    // Each flag picks a source; with none, all sources run.
    let flag = |name: &str| args.iter().any(|arg| arg == name);
    let any_source = flag("--reddit") || flag("--github") || flag("--blog");
    let fetch_reddit = flag("--reddit") || !any_source;
    let fetch_github = flag("--github") || !any_source;
    let fetch_blog = flag("--blog") || !any_source;

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

    // One-off: tell clients which relays this account posts to (NIP-65), then exit.
    if args.iter().any(|arg| arg == "--publish-relay-list") {
        let event_id = publish_relay_list(&nsec, &relays).await?;
        println!(
            "published relay list ({} relays) ({})",
            relays.len(),
            event_id
        );
        return Ok(());
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64();
    let cutoff = now - 3600.0;

    // Fetch and post Reddit content
    if fetch_reddit {
        let client = RedditClient::new()?;
        let posts = client.fetch_rust_posts().await?;

        for post in posts.into_iter().filter(|p| p.created_utc >= cutoff) {
            let text = format!(
                "{}\n\nr/rust by u/{}\n{}",
                post.title, post.author, post.permalink
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
            let stars_today_num = repo
                .stars_today
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
                println!(
                    "dry-run [GitHub]: {}/{} ({} stars today)",
                    repo.author, repo.name, stars_today_num
                );
                println!("{}\n", text);
            } else {
                let event_id = post_nostr(&nsec, &relays, &text).await?;
                println!(
                    "posted [GitHub]: {}/{} ({} stars today) ({})",
                    repo.author, repo.name, stars_today_num, event_id
                );
            }
        }
    }

    // Post new entries from the Rust blogs
    if fetch_blog {
        for source in blog::BLOGS {
            let posts = blog::fetch(source).await?;
            let path = blog::seen_path(source);
            match blog::load_seen(&path)? {
                None if dry_run => println!(
                    "dry-run [{}]: first run, would remember {} existing posts",
                    source.label,
                    posts.len()
                ),
                None => {
                    // First run: remember what's already there instead of posting old announcements.
                    let ids: Vec<&str> = posts.iter().map(|p| p.id.as_str()).collect();
                    blog::remember(&path, &ids)?;
                    println!(
                        "first run [{}]: remembered {} existing posts",
                        source.label,
                        posts.len()
                    );
                }
                Some(seen) => {
                    for post in blog::unseen(&posts, &seen) {
                        let text = format!("📰 {}\n\n{}\n{}", source.label, post.title, post.url);
                        if dry_run {
                            println!("dry-run [{}]: {}", source.label, post.title);
                            println!("{}\n", text);
                        } else {
                            let event_id = post_nostr(&nsec, &relays, &text).await?;
                            blog::remember(&path, &[post.id.as_str()])?;
                            println!("posted [{}]: {} ({})", source.label, post.title, event_id);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
