//! The NBS (Note Block Studio) file format.
//!
//! Turns the format's bytes into scores and back. Everything here describes this
//! one file format, from its wire layout to the musical content it carries.

use crate::axiom::event::{EventMap, Index, Tick};
use crate::{Error, Result};

mod codec;
mod ext;
mod impls;
mod model;

pub use model::*;

// song
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A score as this format stores it: header, events, layers, and custom sounds.
///
/// Holds events of any type `E`, so a score can carry notes today and other
/// kinds of events later.
///
/// Every event anchors to a position, whose layer selects an entry of `layers`;
/// a tone referring to a custom sound selects an entry of `custom_instruments`.
#[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
pub struct NbsSong<E = Note> {
    /// what the format records besides the events.
    pub header: NbsHeader,
    /// every event, keyed by where it sits in the score.
    pub notes: EventMap<Position, E>,
    /// the layers events may occupy, in index order.
    pub layers: Vec<Layer>,
    /// sounds the score defines itself, in index order.
    pub custom_instruments: Vec<CustomInstrument>,
}

impl<E> NbsSong<E> {
    /// creates a new empty song with default values.
    pub fn new() -> Self
    where
        E: Default,
    {
        Self::default()
    }
}

// header
//
// ++++++++++++============++++++++++++============++++++++++++============

/// The parts of a score this format stores besides its events.
///
/// Keeps what the events cannot express: who made the piece, how it should
/// play, and the format's own bookkeeping.
///
/// A value the events could also determine, such as the length, is a hint and
/// need not match them.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct NbsHeader {
    /// the revision the score was written in.
    pub version: Version,
    /// how many built-in sounds the score assumes.
    pub default_instruments: u8,
    /// how long the piece is, as recorded when it was last written.
    pub song_length: Tick,
    /// how many layers the score had when it was last written.
    pub song_layers: Index,
    /// title of the piece.
    pub title: String,
    /// who made the piece.
    pub author: String,
    /// who made the piece this one is based on, if any.
    pub original_author: String,
    /// free-form description of the piece.
    pub description: String,
    /// playback speed, in ticks per second.
    pub tempo: f32,
    /// whether the editor saved the piece automatically.
    pub auto_save: bool,
    /// interval between automatic saves, in minutes.
    pub auto_save_duration: u32,
    /// beats per bar.
    pub time_signature: u8,
    /// time spent editing the piece, in minutes.
    pub minutes_spent: u32,
    /// editor clicks, kept as part of the piece's history.
    pub left_clicks: u32,
    /// editor clicks, kept as part of the piece's history.
    pub right_clicks: u32,
    /// blocks the piece gained while it was edited.
    pub blocks_added: u32,
    /// blocks the piece lost while it was edited.
    pub blocks_removed: u32,
    /// where the piece originally came from.
    pub song_origin: String,
    /// whether playback repeats.
    pub is_loop: bool,
    /// how many times playback repeats; zero means indefinitely.
    pub max_loop_count: u32,
    /// the point playback returns to when it repeats.
    pub loop_start: Tick,
}

impl Default for NbsHeader {
    fn default() -> Self {
        Self {
            version: Version::default(),
            default_instruments: Version::default().vanilla_instruments(),
            song_length: 0,
            song_layers: 0,
            title: String::new(),
            author: String::new(),
            original_author: String::new(),
            description: String::new(),
            tempo: 10.0,
            auto_save: false,
            auto_save_duration: 10,
            time_signature: 4,
            minutes_spent: 0,
            left_clicks: 0,
            right_clicks: 0,
            blocks_added: 0,
            blocks_removed: 0,
            song_origin: String::new(),
            is_loop: false,
            max_loop_count: 0,
            loop_start: 0,
        }
    }
}

/// A revision of this format.
///
/// Records compatibility, so a score can be read back the way it was written.
///
/// Only revisions this library understands can be built; anything newer is
/// rejected rather than guessed at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u8);

impl Version {
    /// The latest revision supported by this library.
    const LATEST: u8 = 6;

    /// creates a revision, rejecting anything newer than the latest supported
    /// one.
    pub fn new(version: u8) -> Result<Self> {
        match version {
            0..=Self::LATEST => Ok(Self(version)),
            _ => Err(Error::InvalidVersion(version.to_string())),
        }
    }

    /// returns the raw revision number.
    pub const fn get(&self) -> u8 {
        self.0
    }

    /// how many built-in sounds this revision can index.
    ///
    /// Later revisions grew the set; four sounds join it in revision 6.
    pub const fn vanilla_instruments(self) -> u8 {
        match self.0 {
            0 => 10,
            1..=5 => 16,
            _ => 20,
        }
    }
}

impl Default for Version {
    fn default() -> Self {
        Self(Self::LATEST)
    }
}
