use aij_judgend::{app, initialize};
use axum_test::TestServer;

pub async fn get_test_server() -> TestServer {
    let _ = initialize().await;
    let server = TestServer::new(app()).expect("Failed to create test server");
    server
}
