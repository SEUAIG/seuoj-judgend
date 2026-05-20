use crate::error::Result;
use crate::fs;
use crate::server::utils::build_response_from_file_content;
use axum::extract::Path;
use axum::response::IntoResponse;

pub(crate) async fn get_submission_file(
    Path((sid, filename)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    fs::assert_submission_exists(&sid).await?;

    fs::validate_filename(&filename)?;
    let file_path = fs::get_path_by_sid_name(&sid, &filename, true).await?;
    let file_content = fs::get_stream_by_path(file_path).await?;

    build_response_from_file_content(sid, filename, file_content)
}
