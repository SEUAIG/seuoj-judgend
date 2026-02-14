use crate::utils::get_test_server;

mod utils;

#[tokio::test]
async fn test_put_problem_config() {
    let server = get_test_server().await;

    let response = server
        .put("/judge/problem/config/1?type=CASE")
        .text("[[test_cases]]\nin_name = '1.in'\nans_name = '1.ans'\nid = 2\n")
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");

    let response = server
        .put("/judge/problem/config/1?type=CASE")
        .text("[[test_cases]]\nin_name = '1.in'\nans_name = '1.ans'\nid = 1\n")
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
}
