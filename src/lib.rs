//! NBS (Note Block Studio) file format library for Rust.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

pub use error::{Error, Result};

pub mod axiom {
    //! axioms: vocabulary that holds regardless of game or file format.
    //!
    //! Types here describe only how events are anchored and ordered. Anything
    //! that only makes sense for one game lives in [`crate::game`], and anything
    //! that only describes a file lives in [`crate::format`].

    pub mod event;

    pub use event::{EventMap, EventStreamExt, Index, LayerAnchor, Tick, TimeAnchor};
}

pub mod game {
    //! per-game rules: instrument catalogue, key layout, and tick rates.

    pub mod minecraft;
}

pub mod format {
    //! file formats: wire models and the bytes they map to.

    pub mod nbs;

    pub use nbs::{Header, Song, Version};
}

// error
//
// ++++++++++++============++++++++++++============++++++++++++============

pub mod error {
    //! failure modes shared by every layer.

    /// failure modes of constructing, reading, and writing values.
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        /// the requested nbs version is not supported.
        #[error("Invalid or unsupported version: {0}")]
        InvalidVersion(String),
        /// the panning is outside `-100..=100`.
        #[error("Invalid panning value: {0} (must be -100 to 100)")]
        InvalidPanning(String),
        /// the volume is above `100`.
        #[error("Invalid velocity value: {0} (must be 0-100)")]
        InvalidVolume(String),
        /// an I/O error occurred while reading or writing.
        #[error(transparent)]
        Io(#[from] std::io::Error),
    }

    /// result type for fallible operations.
    pub type Result<T> = std::result::Result<T, Error>;
}
