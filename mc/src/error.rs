use thiserror::Error;

#[derive(Debug, Error)]
pub enum McError {
    #[error("i/o: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid .graph: {0}")]
    InvalidGraph(String),

    #[error("frontmatter parse: {0}")]
    Frontmatter(#[from] toml::de::Error),

    #[error("missing required section `{0}`")]
    MissingSection(&'static str),

    #[error("conformance check failed: {0}")]
    Conformance(String),
}

pub type Result<T> = std::result::Result<T, McError>;
