use crate::utils::get_test_server;

mod utils;

#[tokio::test]
async fn test_get_problem_tree() {
    let server = get_test_server().await;

    let response = server.get("/judge/problem/tree/test01").await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
    assert_eq!(json["data"]["tree"].as_array().unwrap().len(), 3);
}
