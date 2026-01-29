use std::path::Path;

/// Make the file at `path` executable by adding execute permissions for user, group, and others.
pub(crate) async fn chmod_plus_x(path: impl AsRef<Path>) -> tokio::io::Result<()> {
    #[cfg(unix)]
    {
        let metadata = tokio::fs::metadata(&path).await?;
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        if permissions.mode() & 0o111 == 0 {
            permissions.set_mode(permissions.mode() | 0o111);
            return tokio::fs::set_permissions(&path, permissions).await;
        }
    }
    Ok(())
}
