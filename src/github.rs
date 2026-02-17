use anyhow::{Context, Result};
use scraper::{Html, Selector};

pub struct GitHubClient {
    http: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct TrendingRepo {
    pub name: String,
    pub author: String,
    pub url: String,
    pub description: String,
    pub language: String,
    pub stars: String,
    pub forks: String,
    pub stars_today: String,
}

impl GitHubClient {
    pub fn new() -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .build()
            .context("build reqwest client")?;
        Ok(Self { http })
    }

    pub async fn fetch_trending(&self, language: &str, since: &str) -> Result<Vec<TrendingRepo>> {
        let url = format!(
            "https://github.com/trending/{}?since={}",
            language, since
        );

        let html = self
            .http
            .get(&url)
            .send()
            .await
            .context("request trending page")?
            .error_for_status()
            .context("trending page returned error status")?
            .text()
            .await
            .context("read trending page html")?;

        self.parse_trending_html(&html)
    }

    fn parse_trending_html(&self, html: &str) -> Result<Vec<TrendingRepo>> {
        let document = Html::parse_document(html);

        // GitHub trending uses article.Box-row for each repository
        let article_selector = Selector::parse("article.Box-row").unwrap();
        let h2_selector = Selector::parse("h2 a").unwrap();
        let desc_selector = Selector::parse("p").unwrap();
        let lang_selector = Selector::parse("span[itemprop='programmingLanguage']").unwrap();
        let star_selector = Selector::parse("a[href$='/stargazers']").unwrap();
        let fork_selector = Selector::parse("a[href$='/forks']").unwrap();
        let stars_today_selector = Selector::parse("span.d-inline-block.float-sm-right").unwrap();

        let mut repos = Vec::new();

        for article in document.select(&article_selector) {
            // Extract repository name and author
            let h2_element = article.select(&h2_selector).next();
            if h2_element.is_none() {
                continue;
            }

            let h2 = h2_element.unwrap();
            let href = h2.value().attr("href").unwrap_or("");
            let parts: Vec<&str> = href.trim_matches('/').split('/').collect();

            if parts.len() < 2 {
                continue;
            }

            let author = parts[0].to_string();
            let name = parts[1].to_string();
            let url = format!("https://github.com{}", href);

            // Extract description
            let description = article
                .select(&desc_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract language
            let language = article
                .select(&lang_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract stars
            let stars = article
                .select(&star_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract forks
            let forks = article
                .select(&fork_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            // Extract stars today
            let stars_today = article
                .select(&stars_today_selector)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();

            repos.push(TrendingRepo {
                name,
                author,
                url,
                description,
                language,
                stars,
                forks,
                stars_today,
            });
        }

        Ok(repos)
    }
}
