use aij_judgend::{app, initialize};
use axum_test::TestServer;

pub async fn get_test_server() -> TestServer {
    let _ = initialize().await;
    TestServer::new(app())
}
