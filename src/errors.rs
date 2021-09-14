use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConstellationError {
    #[error("ResponseError HTTP(s) Error:{0}")]
    _ResponseError(String),
    #[error(transparent)]
    URLParseError(#[from] url::ParseError),
}
