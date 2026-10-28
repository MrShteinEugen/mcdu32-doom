// SPDX-License-Identifier: GPL-3.0-only
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid font map: {0}")]
    InvalidMap(String),
    #[error("invalid screen cell {0}")]
    InvalidCell(usize),
}

pub(super) fn invalid(message: impl Into<String>) -> ProtocolError {
    ProtocolError::InvalidMap(message.into())
}
