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
        id: "abc123".to_string(),
        title: "Test Post".to_string(),
        author: "test_user".to_string(),
        permalink: "/r/rust/comments/abc123/test_post/".to_string(),
        url: "https://example.com".to_string(),
        score: 42,
        num_comments: 10,
        created_utc: 1234567890.0,
    };

    assert_eq!(post.id, "abc123");
    assert_eq!(post.title, "Test Post");
    assert_eq!(post.author, "test_user");
    assert_eq!(post.score, 42);
    assert_eq!(post.num_comments, 10);
}
