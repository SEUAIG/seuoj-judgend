use aij_judgend::app;
use aij_judgend::config::AijConfig;
use aij_judgend::logger::init_logger;
use axum_test::TestServer;
use base64::engine::general_purpose;
use base64::Engine;
use serde_json::json;

#[tokio::test]
async fn test_edit_problem() {
    let config = AijConfig::get();
    let _guard = init_logger(&config.log_dir);
    let server = TestServer::new(app()).expect("Failed to create test server");

    let checker_path = "./assets/problems/1/checker";
    let base64_checker = general_purpose::STANDARD.encode(std::fs::read(checker_path).expect("Failed to read checker file"));

    let response = server
        .patch("/judge/problem/edit")
        .json(&json!({
            "pid": "1",
            "checker": {"type": "Binary", "data": base64_checker},
        }))
        .await;
    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
}