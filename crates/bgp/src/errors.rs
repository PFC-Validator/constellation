use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConstellationBGPError {
    #[error("Bad IP ? {0}")]
    BadIp(String),
}
