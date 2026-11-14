//! One error type for the engine. It serialises as `{ "kind": "...", "message": "..." }`, which is what the
//! window receives when a command fails.

use serde::ser::SerializeStruct;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// The caller passed something that cannot be used (bad path, unknown id, out-of-range value).
    InvalidInput(String),
    /// Something that should exist does not (a folder, a history batch).
    NotFound(String),
    /// An operating-system call failed.
    Io {
        context: String,
        source: std::io::Error,
    },
    /// The user cancelled.
    Cancelled,
    /// Another scan or rename is already running.
    Busy(String),
    /// A stored file could not be read or written as data.
    Data(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Error::Io {
            context: context.into(),
            source,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Error::InvalidInput(_) => "invalid_input",
            Error::NotFound(_) => "not_found",
            Error::Io { .. } => "io",
            Error::Cancelled => "cancelled",
            Error::Busy(_) => "busy",
            Error::Data(_) => "data",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidInput(m) | Error::NotFound(m) | Error::Busy(m) | Error::Data(m) => {
                f.write_str(m)
            }
            Error::Io { context, source } => {
                write!(
                    f,
                    "{context}: {}",
                    crate::filesystem::describe_io_error(source)
                )
            }
            Error::Cancelled => f.write_str("Cancelled"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Data(format!("Stored data is damaged: {e}"))
    }
}

impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        let mut st = s.serialize_struct("Error", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}
