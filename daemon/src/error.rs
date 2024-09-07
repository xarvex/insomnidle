use thiserror::Error;
use tokio::io;

use crate::inhibitor;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Inhibitor(#[from] inhibitor::error::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
}
