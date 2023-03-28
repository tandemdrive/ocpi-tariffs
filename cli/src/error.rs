use std::fmt::Display;
use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File error `{path}`: {error}")]
    File { path: PathBuf, error: io::Error },
    #[error("Could not deserialize {kind} from `{path}`: {error}")]
    Deserialize {
        path: String,
        kind: &'static str,
        error: serde_json::Error,
    },
    #[error("Invalid timezone `{0}`")]
    Timezone(String),
    #[error("{0:?}")]
    Internal(ocpi_tariffs::Error),
}

impl Error {
    pub fn file(path: PathBuf, error: io::Error) -> Self {
        Self::File { path, error }
    }

    pub fn deserialize(path: impl Display, kind: &'static str, error: serde_json::Error) -> Self {
        Self::Deserialize {
            path: path.to_string(),
            kind,
            error,
        }
    }
}
