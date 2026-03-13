use thiserror::Error;

#[derive(Error, Debug)]
pub enum OntologyError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("SPARQL error: {0}")]
    Sparql(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("alignment error: {0}")]
    Alignment(String),

    #[error("reasoning error: {0}")]
    Reasoning(String),

    #[error("configuration error: {0}")]
    Config(String),

    #[error("feature not enabled: {0}")]
    FeatureDisabled(String),
}

pub type Result<T> = std::result::Result<T, OntologyError>;
