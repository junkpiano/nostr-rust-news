use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// A blog to follow. `state` names the file that lists the posts already sent.
pub struct Blog {
    pub label: &'static str,
    pub feed_url: &'static str,
    pub state: &'static str,
}

pub const BLOGS: &[Blog] = &[
    Blog {
        label: "Rust Blog",
        feed_url: "https://blog.rust-lang.org/feed.xml",
        state: "blog-seen",
    },
    Blog {
        label: "Inside Rust",
        feed_url: "https://blog.rust-lang.org/inside-rust/feed.xml",
        state: "inside-rust-seen",
    },
];

#[derive(Debug, Clone, PartialEq)]
pub struct BlogPost {
    pub id: String,
    pub title: String,
    pub url: String,
}

/// Posts in a blog's feed, newest first.
pub async fn fetch(blog: &Blog) -> Result<Vec<BlogPost>> {
    let response = reqwest::get(blog.feed_url)
        .await
        .with_context(|| format!("request {} feed", blog.label))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("read {} feed", blog.label))?;
    if !status.is_success() {
        bail!("{} feed returned status={status}", blog.label);
    }
    parse_blog_feed(&body)
}

pub fn parse_blog_feed(xml: &str) -> Result<Vec<BlogPost>> {
    let feed: AtomFeed = quick_xml::de::from_str(xml).context("parse blog atom feed")?;
    Ok(feed
        .entries
        .into_iter()
        .map(|e| {
            let url = e
                .links
                .iter()
                .find(|l| l.rel.as_deref().unwrap_or("alternate") == "alternate")
                .map(|l| l.href.clone())
                .unwrap_or_else(|| e.id.clone());
            BlogPost {
                id: e.id,
                title: e.title,
                url,
            }
        })
        .collect())
}

/// Where the IDs of a blog's posts already sent are kept, one per line.
/// The blogs date posts at midnight, so a time window can't tell new posts apart.
pub fn seen_path(blog: &Blog) -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        });
    base.join("nostr-rust-news").join(blog.state)
}

/// The IDs already sent, or None if nothing has been recorded yet (the first run).
pub fn load_seen(path: &Path) -> Result<Option<HashSet<String>>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text.lines().map(str::to_string).collect())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}

pub fn remember(path: &Path, ids: &[&str]) -> Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    for id in ids {
        writeln!(file, "{id}")?;
    }
    Ok(())
}

/// Posts not sent yet, oldest first.
pub fn unseen(posts: &[BlogPost], seen: &HashSet<String>) -> Vec<BlogPost> {
    posts
        .iter()
        .rev()
        .filter(|p| !seen.contains(&p.id))
        .cloned()
        .collect()
}

#[derive(serde::Deserialize)]
struct AtomFeed {
    #[serde(rename = "entry", default)]
    entries: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct Entry {
    id: String,
    title: String,
    #[serde(rename = "link", default)]
    links: Vec<Link>,
}

#[derive(serde::Deserialize)]
struct Link {
    #[serde(rename = "@href")]
    href: String,
    #[serde(rename = "@rel")]
    rel: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Rust Blog</title>
  <entry>
    <title>Newer</title>
    <link rel="alternate" href="https://blog.rust-lang.org/2026/10/02/newer/" type="text/html" />
    <published>2026-10-02T00:00:00+00:00</published>
    <id>https://blog.rust-lang.org/2026/10/02/newer/</id>
  </entry>
  <entry>
    <title>Older</title>
    <link rel="alternate" href="https://blog.rust-lang.org/2026/09/01/older/" type="text/html" />
    <id>https://blog.rust-lang.org/2026/09/01/older/</id>
  </entry>
</feed>"#;

    #[test]
    fn parses_the_feed_and_picks_unseen_posts_oldest_first() {
        let posts = parse_blog_feed(XML).unwrap();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].title, "Newer");
        assert_eq!(posts[0].url, "https://blog.rust-lang.org/2026/10/02/newer/");

        let none_seen = HashSet::new();
        let order: Vec<_> = unseen(&posts, &none_seen)
            .into_iter()
            .map(|p| p.title)
            .collect();
        assert_eq!(order, ["Older", "Newer"]);

        let seen: HashSet<String> = [posts[1].id.clone()].into();
        assert_eq!(unseen(&posts, &seen), vec![posts[0].clone()]);
    }

    #[test]
    fn remembers_ids_across_loads() {
        let path = std::env::temp_dir().join(format!("blog-seen-test-{}", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert_eq!(load_seen(&path).unwrap(), None);
        remember(&path, &["a", "b"]).unwrap();
        remember(&path, &["c"]).unwrap();
        let seen = load_seen(&path).unwrap().unwrap();
        assert_eq!(seen.len(), 3);
        assert!(seen.contains("c"));
        std::fs::remove_file(&path).unwrap();
    }
}
