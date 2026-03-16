use crate::utils::get_test_server;
use serde_json::json;

mod utils;
#[tokio::test]
async fn test_edit_problem() {
    let server = get_test_server().await;

    let response = server
        .patch("/judge/problem/edit")
        .json(&json!({
            "pid": "p01",
            "description": "Updated problem description",
        }))
        .await;
    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
}

#[tokio::test]
async fn test_delete_problem() {
    let server = get_test_server().await;

    let response = server.delete("/judge/problem/ABC").await;
    response.assert_status_no_content();
}
