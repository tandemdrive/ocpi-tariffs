use std::{fmt, io, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    File {
        path: PathBuf,
        error: io::Error,
    },
    Deserialize {
        path: String,
        kind: &'static str,
        error: std::io::Error,
    },
    Internal(ocpi_tariffs::Error),
}
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::File { path, error } => {
                format!("File error `{}`: {}", path.display(), error)
            }
            Self::Deserialize { path, kind, error } => {
                format!("Could not deserialize {kind} from `{path}`: {error}")
            }
            Self::Internal(e) => format!("{e}"),
        };

        f.write_str(&s)
    }
}

impl Error {
    pub fn file(path: PathBuf, error: io::Error) -> Self {
        Self::File { path, error }
    }

    pub fn deserialize(
        path: impl fmt::Display,
        kind: &'static str,
        error: impl Into<std::io::Error>,
    ) -> Self {
        Self::Deserialize {
            path: path.to_string(),
            kind,
            error: error.into(),
        }
    }
}
