use crate::utils::get_test_server;

mod utils;

#[tokio::test]
async fn test_put_problem_config() {
    let server = get_test_server().await;

    let response = server
        .put("/judge/problem/config/1?type=META")
        .text("{\"pid\":\"1\",\"description\":\"\",\"input\":\"一行两个正整数a, b($1 \\\\leq a, b \\\\leq 10^6$)。\",\"output\":\"一行一个正整数 $a+b$。\",\"hint\":\"\",\"example\":[{\"in\":\"1 2\",\"ans\":\"3\",\"description\":\"\"}]}")
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");

    let response = server
        .put("/judge/problem/config/1?type=META")
        .text("{\"pid\":\"1\",\"description\":\"两数之和\",\"input\":\"一行两个正整数a, b($1 \\\\leq a, b \\\\leq 10^6$)。\",\"output\":\"一行一个正整数 $a+b$。\",\"hint\":\"\",\"example\":[{\"in\":\"1 2\",\"ans\":\"3\",\"description\":\"\"}]}")
        .await;

    response.assert_status_ok();
    let body = response.text();
    let json: serde_json::Value = serde_json::from_str(&body).expect("Response is not valid JSON");
    println!("Response JSON: {}", json);
    assert_eq!(json["code"], 0);
    assert_eq!(json["message"], "Success");
}
