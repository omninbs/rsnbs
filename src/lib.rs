//! NBS (Note Block Studio) file format library for Rust.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

mod codec;
#[cfg(test)]
mod tests;

pub mod event;
pub mod note;
pub mod song;
pub mod types;
