use crate::utils::get_test_server;

mod utils;

#[tokio::test]
async fn test_get_case_metadata() {
    let server = get_test_server().await;

    let response = server.get("/judge/problem/data/1").await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
    assert!(json["data"]["test_cases"].is_array());
    assert_eq!(json["data"]["test_cases"].as_array().unwrap().len(), 1);
    assert_eq!(json["data"]["test_cases"][0]["in_name"], "1.in");
    assert_eq!(json["data"]["test_cases"][0]["ans_name"], "1.ans");
}
