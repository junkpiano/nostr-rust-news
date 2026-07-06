use nostr_rust_news::client::{RedditClient, RedditPost};

#[test]
fn test_reddit_client_creation() {
    let client = RedditClient::new();
    assert!(client.is_ok());
}

#[test]
fn test_reddit_post_structure() {
    // Test that RedditPost can be created with all fields
    let post = RedditPost {
        id: "t3_abc123".to_string(),
        title: "Test Post".to_string(),
        author: "test_user".to_string(),
        permalink: "https://www.reddit.com/r/rust/comments/abc123/test_post/".to_string(),
        created_utc: 1234567890.0,
    };

    assert_eq!(post.id, "t3_abc123");
    assert_eq!(post.title, "Test Post");
    assert_eq!(post.author, "test_user");
    assert_eq!(post.created_utc, 1234567890.0);
}
