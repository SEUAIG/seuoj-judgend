use crate::error::{AijError, Result};
use crate::fs;
use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

pub(crate) async fn serve_data_metadata(Path(pid): Path<String>) -> Result<impl IntoResponse> {
    let output_json = fs::read_file_by_id_name(&pid, "case.json").await?;
    let output_json = serde_json::from_str::<serde_json::Value>(&output_json).map_err(|e| {
        AijError::FileSystem(
            StatusCode::INTERNAL_SERVER_ERROR,
            "FAILED_TO_PARSE_TESTCASE_METADATA".to_string(),
            format!("Failed to parse testcase metadata JSON: {}", e),
        )
    })?;
    Ok(Json(json!({
        "code": 0,
        "message": "Success",
        "data": {
            "test_cases": output_json
        }
    })))
}
