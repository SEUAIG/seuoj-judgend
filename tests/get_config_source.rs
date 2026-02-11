use crate::utils::get_test_server;

mod utils;

#[tokio::test]
async fn test_get_config_source() {
    let server = get_test_server().await;

    let response = server.get("/judge/problem/config/1?type=CASE").await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
    let case_config_content = std::fs::read_to_string("./assets/problems/1/data/case.toml")
        .expect("Failed to read case.toml");
    assert_eq!(
        json["data"]["config"].as_str().unwrap(),
        case_config_content
    );
}
