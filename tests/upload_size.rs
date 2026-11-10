use crate::utils::get_test_server;
use axum::http::StatusCode;
use axum_test::multipart::{MultipartForm, Part};

mod utils;

/// Both the file cap and the multipart envelope cap must reject without
/// replacing the existing fixture data directory.
#[tokio::test]
async fn oversized_archives_preserve_existing_data() {
    let server = get_test_server().await;
    let original = std::fs::read("assets/problems/test01/data/1.in").unwrap();
    for size in [200 * 1024 * 1024 + 1, 201 * 1024 * 1024 + 1] {
        let form = MultipartForm::new()
            .add_part("format", Part::text("zip"))
            .add_part(
                "file",
                Part::bytes(vec![0_u8; size]).file_name("oversized.zip"),
            );
        server
            .post("/judge/problem/data/test01")
            .multipart(form)
            .await
            .assert_status(StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(
            std::fs::read("assets/problems/test01/data/1.in").unwrap(),
            original
        );
    }
}
