use anyhow::{Context, Result};

pub struct RedditClient {
    http: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct RedditPost {
    pub id: String,
    pub title: String,
    pub author: String,
    pub permalink: String,
    pub url: String,
    pub score: i64,
    pub num_comments: i64,
    pub created_utc: f64,
}

impl RedditClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent("nostr-rust-news/0.1 (https://www.reddit.com/r/rust.json)")
            .build()
            .context("build reqwest client")?;
        Ok(Self { http })
    }

    pub async fn fetch_rust_posts(&self) -> Result<Vec<RedditPost>> {
        self.fetch_subreddit_posts("rust").await
    }

    pub async fn fetch_subreddit_posts(&self, subreddit: &str) -> Result<Vec<RedditPost>> {
        let url = format!("https://www.reddit.com/r/{subreddit}.json");
        let listing: RedditListing = self
            .http
            .get(url)
            .send()
            .await
            .context("request subreddit listing")?
            .error_for_status()
            .context("subreddit listing returned error status")?
            .json()
            .await
            .context("parse subreddit listing json")?;

        let posts = listing
            .data
            .children
            .into_iter()
            .map(|child| child.data.into())
            .collect();

        Ok(posts)
    }
}

#[derive(serde::Deserialize)]
struct RedditListing {
    data: RedditListingData,
}

#[derive(serde::Deserialize)]
struct RedditListingData {
    children: Vec<RedditChild>,
}

#[derive(serde::Deserialize)]
struct RedditChild {
    data: RedditPostData,
}

#[derive(serde::Deserialize)]
struct RedditPostData {
    id: String,
    title: String,
    author: String,
    permalink: String,
    url: String,
    score: i64,
    num_comments: i64,
    created_utc: f64,
}

impl From<RedditPostData> for RedditPost {
    fn from(value: RedditPostData) -> Self {
        Self {
            id: value.id,
            title: value.title,
            author: value.author,
            permalink: value.permalink,
            url: value.url,
            score: value.score,
            num_comments: value.num_comments,
            created_utc: value.created_utc,
        }
    }
}
