use aij_judgend::app;
use aij_judgend::config::AijConfig;
use aij_judgend::fs::FileSystem;
use aij_judgend::logger::init_logger;
use axum_test::TestServer;
use serde_json::json;

#[tokio::test]
async fn test_submission() {
    let config = AijConfig::init()
        .await
        .expect("Failed to initialize config");
    let _guard = init_logger(&config.log_dir);
    FileSystem::init(&config.problems_dir).expect("Failed to initialize file system");
    let server = TestServer::new(app()).expect("Failed to create test server");

    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "123",
            "pid": "1",
            "code": "#include<bits/stdc++.h>\nint main(){std::cout << 3 << std::endl;}",
            "language": "Cpp"
        }))
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");

    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "111",
            "pid": "2",
            "code": "#include<bits/stdc++.h>\nint main(){int n; std::cin>>n; while(n--){int a, b; std::cin >> a >> b;std::cout << a + b << std::endl;}}",
            "language": "Cpp"
        }))
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
    // waiting logger
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}
