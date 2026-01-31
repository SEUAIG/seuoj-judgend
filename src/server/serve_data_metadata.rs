use crate::error::Result;
use crate::fs;
use axum::extract::Path;
use axum::response::IntoResponse;

pub(crate) async fn serve_data_metadata(Path(pid): Path<String>) -> Result<impl IntoResponse> {
    let output_json = fs::read_file_by_id_name(&pid, "case.json").await?;
    Ok(output_json)
}
