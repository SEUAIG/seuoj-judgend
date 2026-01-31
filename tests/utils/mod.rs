use aij_judgend::app;
use aij_judgend::config::AijConfig;
use aij_judgend::logger::init_logger;
use axum_test::TestServer;

pub fn get_test_server() -> TestServer {
    let config = AijConfig::get();
    init_logger(&config.log_dir);
    let server = TestServer::new(app()).expect("Failed to create test server");
    server
}
