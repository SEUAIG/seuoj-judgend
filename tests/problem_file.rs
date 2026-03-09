use crate::utils::get_test_server;
use axum::http::StatusCode;
mod utils;

#[tokio::test]
async fn test_get_problem_file() {
    let server = get_test_server().await;

    let response = server.get("/judge/problem/file/p01/1.in").await;
    response.assert_status(StatusCode::OK);
    response.assert_header("content-disposition", "attachment; filename=\"1.in\"");
    response.assert_header("content-type", "application/octet-stream");

    let body = response.as_bytes();
    let body_str = String::from_utf8_lossy(&body);
    assert_eq!(body_str, "1 2");
}

#[tokio::test]
async fn test_get_problem_file_not_found() {
    let server = get_test_server().await;
    let response = server.get("/judge/problem/file/999/999.in").await;
    response.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_problem_file_invalid_filename() {
    let server = get_test_server().await;
    let response = server.get("/judge/problem/file/p01/&&.in").await;
    response.assert_status(StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_get_problem_file_zip() {
    let server = get_test_server().await;

    let response = server.get("/judge/problem/file/p01/data.zip").await;
    response.assert_status(StatusCode::OK);
    response.assert_header("content-disposition", "attachment; filename=\"data.zip\"");
    response.assert_header("content-type", "application/octet-stream");

    let body = response.as_bytes();
    assert!(!body.is_empty(), "Expected non-empty zip file content");
}
