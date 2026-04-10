use crate::utils::get_test_server;
use serde_json::json;

mod utils;

#[tokio::test]
async fn test_put_problem_config() {
    let server = get_test_server().await;

    let response = server
        .put("/judge/problem/config/test01")
        .json(&json!({
        "problem_info": {
            "problem_type": "special",
            "checker_type": "special"
        },
        "testcases": [
            {
                "id": 1,
                "in_path": "1.in",
                "ans_path": "1.ans"
            }
        ],
        "custom_modules": {
            "checker_path": "checker.cpp"
        }
            }))
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
}
