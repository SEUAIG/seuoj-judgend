use aij_judgend::app;
use aij_judgend::config::AijConfig;
use aij_judgend::logger::init_logger;
use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_upload_problem_data() {
    let config = AijConfig::get();
    let _guard = init_logger(&config.log_dir);
    let server = TestServer::new(app()).expect("Failed to create test server");


    let response = server.post("/judge/problem/data")
        .json(&json!({
            "pid": "1",
            "testcase": [
                {
                    "id": 1,
                    "in": "1 2\n",
                    "ans": "3\n"
                },
                {
                    "id": 2,
                    "in": "2 3\n",
                    "ans": "5\n"
                }
            ]
        }))
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");

    let response = server.post("/judge/problem/data")
        .json(&json!({
            "pid": "1",
            "testcase": [
                {
                    "id": 1,
                    "in": "1 2",
                    "ans": "3"
                }
            ]
        }))
        .await;
    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
}