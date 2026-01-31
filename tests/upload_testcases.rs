use crate::utils::get_test_server;
use serde_json::json;

mod utils;

#[tokio::test]
async fn test_upload_problem_data() {
    let server = get_test_server().await;

    let response = server
        .post("/judge/problem/data")
        .json(&json!({
            "pid": "1",
            "testcase": [
                {
                    "id": 1,
                    "in": "1 2\n",
                    "in_name": "1.aaa",
                    "ans": "3\n",
                    "ans_name": "1.222"
                },
                {
                    "id": 2,
                    "in": "2 3\n",
                    "in_name": "2.xxx",
                    "ans": "5\n",
                    "ans_name": "2.uuu"
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

    let response = server
        .post("/judge/problem/data")
        .json(&json!({
            "pid": "1",
            "testcase": [
                {
                    "id": 1,
                    "in": "1 2",
                    "in_name": "1.in",
                    "ans": "3",
                    "ans_name": "1.ans"
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
