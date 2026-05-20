use crate::utils::get_test_server;
use serde_json::json;

mod utils;
#[tokio::test]
async fn test_submission() {
    let server = get_test_server().await;

    let response = server
        .post("/judge/submission")
        .json(&json!({
            "submissionId": "123",
            "pid": "test01",
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
            "pid": "test02",
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
