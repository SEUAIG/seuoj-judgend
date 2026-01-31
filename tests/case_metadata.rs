use crate::utils::get_test_server;

mod utils;

#[tokio::test]
async fn test_get_case_metadata() {
    let server = get_test_server();

    let response = server.get("/judge/problem/data/1").await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    let json_array = json.as_array().expect("Response JSON is not an array");
    assert_eq!(json_array.len(), 1);
    let case_metadata = &json_array[0];
    assert_eq!(case_metadata["id"], 1);
    assert_eq!(case_metadata["in_name"], "1.in");
    assert_eq!(case_metadata["ans_name"], "1.ans");
}
