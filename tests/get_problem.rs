use aij_judgend::app;
use aij_judgend::config::AijConfig;
use aij_judgend::logger::init_logger;
use axum_test::TestServer;

#[tokio::test]
async fn test_get_problem() {
    let config = AijConfig::get().await.expect("Failed to initialize config");
    let _guard = init_logger(&config.log_dir);
    let server = TestServer::new(app()).expect("Failed to create test server");

    let response = server.get("/judge/problem/1").await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
    assert_eq!(json["data"]["example"].as_array().unwrap().len(), 1);
}
