//! Error type and process exit codes.

use std::fmt;

/// Exit codes are part of the CLI contract and are asserted by tests.
pub const EXIT_OK: i32 = 0;
pub const EXIT_PARTIAL: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_EMPTY: i32 = 3;
pub const EXIT_STORE: i32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Usage,
    Store,
    Empty,
    Partial,
}

#[derive(Debug, Clone)]
pub struct AppError {
    kind: Kind,
    message: String,
}

impl AppError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            kind: Kind::Usage,
            message: message.into(),
        }
    }

    pub fn store(message: impl Into<String>) -> Self {
        Self {
            kind: Kind::Store,
            message: message.into(),
        }
    }

    pub fn empty(message: impl Into<String>) -> Self {
        Self {
            kind: Kind::Empty,
            message: message.into(),
        }
    }

    pub fn partial(message: impl Into<String>) -> Self {
        Self {
            kind: Kind::Partial,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn exit_code(&self) -> i32 {
        match self.kind {
            Kind::Usage => EXIT_USAGE,
            Kind::Store => EXIT_STORE,
            Kind::Empty => EXIT_EMPTY,
            Kind::Partial => EXIT_PARTIAL,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}
