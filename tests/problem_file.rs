use aij_judgend::app;
use aij_judgend::config::AijConfig;
use aij_judgend::logger::init_logger;
use axum::http::StatusCode;
use axum_test::TestServer;

#[tokio::test]
async fn test_get_problem_file() {
    let config = AijConfig::get();
    let _guard = init_logger(&config.log_dir);
    let server = TestServer::new(app()).expect("Failed to create test server");

    let response = server.get("/judge/problem/file/1/1.in").await;
    response.assert_status(StatusCode::OK);
    response.assert_header("content-disposition", "attachment; filename=\"1.in\"");
    response.assert_header("content-type", "application/octet-stream");

    let body = response.as_bytes();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(body_str, "1 2");
}

#[tokio::test]
async fn test_get_problem_file_not_found() {
    let config = AijConfig::get();
    let _guard = init_logger(&config.log_dir);
    let server = TestServer::new(app()).expect("Failed to create test server");

    let response = server.get("/judge/problem/file/999/999.in").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_problem_file_invalid_filename() {
    let config = AijConfig::get();
    let _guard = init_logger(&config.log_dir);
    let server = TestServer::new(app()).expect("Failed to create test server");

    let response = server.get("/judge/problem/file/1/&&.in").await;
    response.assert_status(StatusCode::BAD_REQUEST);
}
