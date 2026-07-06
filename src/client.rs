use anyhow::{bail, Context, Result};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub struct RedditClient {
    http: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct RedditPost {
    pub id: String,
    pub title: String,
    pub author: String,
    pub permalink: String,
    pub created_utc: f64,
}

impl RedditClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .http1_only()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            )
            .build()
            .context("build reqwest client")?;
        Ok(Self { http })
    }

    pub async fn fetch_rust_posts(&self) -> Result<Vec<RedditPost>> {
        self.fetch_subreddit_posts("rust").await
    }

    pub async fn fetch_subreddit_posts(&self, subreddit: &str) -> Result<Vec<RedditPost>> {
        // The unauthenticated .json listing endpoints now return 403, so we
        // consume the Atom feed instead.
        let url = format!("https://www.reddit.com/r/{subreddit}/.rss");
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("request subreddit feed")?;
        let status = response.status();
        let body = response
            .text()
            .await
            .context("read subreddit feed response body")?;
        if !status.is_success() {
            let body_head: String = body.chars().take(200).collect();
            bail!("subreddit feed returned status={status} body_head={body_head:?}");
        }

        parse_atom_feed(&body)
    }
}

fn parse_atom_feed(xml: &str) -> Result<Vec<RedditPost>> {
    let feed: AtomFeed = quick_xml::de::from_str(xml).context("parse subreddit atom feed")?;

    feed.entries
        .into_iter()
        .map(|entry| {
            let created_utc = OffsetDateTime::parse(&entry.published, &Rfc3339)
                .with_context(|| format!("parse published timestamp {:?}", entry.published))?
                .unix_timestamp() as f64;
            let author = entry
                .author
                .map(|a| a.name.trim_start_matches("/u/").to_string())
                .unwrap_or_else(|| "[deleted]".to_string());
            Ok(RedditPost {
                id: entry.id,
                title: entry.title,
                author,
                permalink: entry.link.href,
                created_utc,
            })
        })
        .collect()
}

#[derive(serde::Deserialize)]
struct AtomFeed {
    #[serde(rename = "entry", default)]
    entries: Vec<AtomEntry>,
}

#[derive(serde::Deserialize)]
struct AtomEntry {
    id: String,
    title: String,
    author: Option<AtomAuthor>,
    link: AtomLink,
    published: String,
}

#[derive(serde::Deserialize)]
struct AtomAuthor {
    name: String,
}

#[derive(serde::Deserialize)]
struct AtomLink {
    #[serde(rename = "@href")]
    href: String,
}

#[cfg(test)]
mod tests {
    use super::parse_atom_feed;

    #[test]
    fn parses_reddit_atom_feed() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Rust</title>
  <entry>
    <author><name>/u/testuser</name><uri>https://www.reddit.com/user/testuser</uri></author>
    <category term="rust" label="r/rust"/>
    <content type="html">&lt;p&gt;body&lt;/p&gt;</content>
    <id>t3_abc123</id>
    <link href="https://www.reddit.com/r/rust/comments/abc123/test_post/"/>
    <updated>2026-06-29T11:04:08+00:00</updated>
    <published>2026-06-29T11:04:08+00:00</published>
    <title>Test Post</title>
  </entry>
</feed>"#;

        let posts = parse_atom_feed(xml).expect("feed should parse");
        assert_eq!(posts.len(), 1);
        let post = &posts[0];
        assert_eq!(post.id, "t3_abc123");
        assert_eq!(post.title, "Test Post");
        assert_eq!(post.author, "testuser");
        assert_eq!(
            post.permalink,
            "https://www.reddit.com/r/rust/comments/abc123/test_post/"
        );
        assert_eq!(post.created_utc, 1782731048.0);
    }
}
