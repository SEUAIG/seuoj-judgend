use crate::utils::get_test_server;
use axum_test::multipart::{MultipartForm, Part};

mod utils;

#[tokio::test]
async fn test_upload_problem_data() {
    let server = get_test_server().await;
    std::fs::rename("./assets/problems/test01/data", "./test_backup_data_dir")
        .expect("Failed to copy test input file");
    std::fs::write("./1.in", b"1 2 3\n").expect("Failed to write test input file");

    std::process::Command::new("zip")
        .arg("data.zip")
        .arg("./1.in")
        .output()
        .expect("Failed to create zip file");

    let zip_bytes = std::fs::read("./data.zip").expect("Failed to read zip file");

    let multi_part = MultipartForm::new()
        .add_part("file", Part::bytes(zip_bytes).file_name("data.zip"))
        .add_part("format", Part::text("zip"));
    let response = server
        .post("/judge/problem/data/test01")
        .multipart(multi_part)
        .await;

    response.assert_status_ok();

    let data_1_in_content = std::fs::read_to_string("./assets/problems/test01/data/1.in")
        .expect("Failed to read test input file after upload");
    assert_eq!(data_1_in_content, "1 2 3\n");

    std::fs::remove_file("./1.in").expect("Failed to remove test input file");
    std::fs::remove_file("./data.zip").expect("Failed to remove zip file");
    std::fs::remove_dir_all("./assets/problems/test01/data")
        .expect("Failed to remove test data directory");
    std::fs::rename("./test_backup_data_dir", "./assets/problems/test01/data")
        .expect("Failed to restore test data directory");
}
