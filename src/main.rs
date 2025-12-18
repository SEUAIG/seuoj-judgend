#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
//! SEU AIJ Judge-Endpoint

use crate::config::init_config;
use crate::logger::init_logger;
mod config;
mod logger;
mod error;
mod judger;

use error::Result;


#[tokio::main]
async fn main() -> Result<()> {
    let config = init_config().await?;
    let _guard = init_logger(&config.log_dir);
    Ok(())
}
