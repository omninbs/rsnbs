//! NBS (Note Block Studio) file format library for Rust.

mod codec;
#[cfg(test)]
mod tests;

pub mod note;

// song
//
// ++++++++++++============++++++++++++============++++++++++++============

pub mod song {
    use crate::note::{Note, NoteStreamExt, Notes};
    use crate::types::{Index, Panning, Position, Result, Tick, Version, Volume};

    /// represents a complete nbs song with header, notes, layers, and instruments.
    #[derive(Debug, Default, Clone, PartialEq, PartialOrd)]
    pub struct Song {
        pub header: Header,
        // Position data is stored redundantly due to incremental encoding
        // I don't know why the header's song_length is only u16 :(
        pub notes: Notes<Position, Note>,
        pub layers: Vec<Layer>,
        pub custom_instruments: Vec<CustomInstrument>,
    }

    impl Song {
        /// creates a new empty song with default values.
        pub fn new() -> Self {
            Self::default()
        }

        /// opens and parses an nbs file from a file path
        pub fn open_nbs<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
            let mut file = std::fs::File::open(path)?;
            Self::parse(&mut file)
        }

        /// parses an nbs file from standard input
        pub fn from_stdin() -> Result<Self> {
            let mut stdin = std::io::stdin();
            Self::parse(&mut stdin)
        }

        /// saves the song to an nbs file at the specified path
        pub fn save_nbs<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
            let mut file = std::fs::File::create(path)?;
            self.write(&mut file)
        }

        /// writes the song to standard output
        pub fn to_stdout(&mut self) -> Result<()> {
            let mut stdout = std::io::stdout();
            self.write(&mut stdout)
        }

        /// Resolves the song's notes against the layers they sit on, yielding
        /// how each will actually sound.
        pub fn resolved_notes(self) -> impl Iterator<Item = (Position, Note)> {
            let Self { notes, layers, .. } = self;
            notes.into_iter().apply_layers(layers)
        }

        /// returns the tick count (max tick + 1) of the song.
        pub fn len(&self) -> Tick {
            self.notes.tick_count()
        }
    }

    // header
    //
    // ++++++++++++============++++++++++++============++++++++++++============

    /// contains metadata and song information from the nbs file header.
    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    pub struct Header {
        pub version: Version,
        pub default_instruments: u8,
        pub song_length: Tick,
        pub song_layers: Index,
        pub song_name: String,
        pub song_author: String,
        pub original_author: String,
        pub description: String,
        pub tempo: f32,
        pub auto_save: bool,
        pub auto_save_duration: u32,
        pub time_signature: u8,
        pub minutes_spent: u32,
        pub left_clicks: u32,
        pub right_clicks: u32,
        pub blocks_added: u32,
        pub blocks_removed: u32,
        pub song_origin: String,
        pub is_loop: bool,
        pub max_loop_count: u32,
        pub loop_start: Tick,
    }

    impl Default for Header {
        fn default() -> Self {
            Self {
                version: Version::default(),
                default_instruments: Version::default().vanilla_instruments(),
                song_length: 0,
                song_layers: 0,
                song_name: String::new(),
                song_author: String::new(),
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

    // layer
    //
    // ++++++++++++============++++++++++++============++++++++++++============

    /// represents a layer with a name, volume, panning, and a disabled flag.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Layer {
        pub name: String,
        pub disabled: bool,
        pub volume: Volume,
        pub panning: Panning,
    }

    impl Layer {
        /// A neutral layer: unnamed, enabled, full volume, centered panning.
        pub const fn new() -> Self {
            Self {
                name: String::new(),
                disabled: false,
                volume: Volume::FULL,
                panning: Panning::CENTER,
            }
        }
    }

    impl Default for Layer {
        fn default() -> Self {
            Self::new()
        }
    }

    // custom instrument
    //
    // ++++++++++++============++++++++++++============++++++++++++============

    /// represents an instrument with sound file, pitch, and playback settings.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct CustomInstrument {
        pub name: String,
        pub file: String,
        pub pitch: u8,
        pub press_key: bool,
    }

    impl Default for CustomInstrument {
        fn default() -> Self {
            Self {
                name: String::new(),
                file: String::new(),
                pitch: 45,
                press_key: true,
            }
        }
    }
}

// types
//
// ++++++++++++============++++++++++++============++++++++++++============

pub mod types {
    /// represents a valid nbs version format
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Version(u8);

    impl Version {
        /// The latest NBS file format version supported by this library.
        const LATEST: u8 = 6;

        pub fn new(version: u8) -> Result<Self> {
            match version {
                0..=Self::LATEST => Ok(Self(version)),
                _ => Err(Error::InvalidVersion(version.to_string())),
            }
        }

        pub const fn get(&self) -> u8 {
            self.0
        }

        /// the amount of vanilla instruments in this NBS version.
        /// version 6 added the four trumpet instruments.
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

    // anchors
    //
    // ++++++++++++============++++++++++++============++++++++++++============

    /// Time-axis anchor: reads and rebuilds its tick coordinate.
    pub trait TimeAnchor: Sized + Copy {
        fn into_tick(self) -> Tick;
        fn with_tick(self, tick: Tick) -> Self;
    }

    impl TimeAnchor for Position {
        fn into_tick(self) -> Tick {
            self.tick
        }
        fn with_tick(self, tick: Tick) -> Self {
            Self { tick, ..self }
        }
    }

    impl TimeAnchor for Tick {
        fn into_tick(self) -> Tick {
            self
        }
        fn with_tick(self, tick: Tick) -> Self {
            tick
        }
    }

    /// Layer-axis anchor: reads and rebuilds its layer coordinate.
    pub trait LayerAnchor: Sized + Copy {
        fn into_layer(self) -> Index;
        fn with_layer(self, layer: Index) -> Self;
    }

    impl LayerAnchor for Position {
        fn into_layer(self) -> Index {
            self.layer
        }
        fn with_layer(self, layer: Index) -> Self {
            Self { layer, ..self }
        }
    }

    impl LayerAnchor for Index {
        fn into_layer(self) -> Index {
            self
        }
        fn with_layer(self, layer: Index) -> Self {
            layer
        }
    }

    /// represents a position with tick and layer index.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Position {
        tick: Tick,
        layer: Index,
    }

    impl Position {
        pub const fn new(tick: Tick, layer: Index) -> Self {
            Self { tick, layer }
        }
    }

    // basic
    //
    // ++++++++++++============++++++++++++============++++++++++++============

    /// represents volume value in range 0-100
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Volume(u8);

    impl Default for Volume {
        fn default() -> Self {
            Self::FULL
        }
    }

    impl Volume {
        /// Full volume (100).
        pub const FULL: Self = Self(100);

        pub fn new(volume: u8) -> Result<Self> {
            match volume {
                0..=100 => Ok(Self(volume)),
                _ => Err(Error::InvalidVolume(volume.to_string())),
            }
        }

        pub const fn get(&self) -> u8 {
            self.0
        }

        /// Combines a note's volume with its layer's, as NBS playback does.
        pub fn combine(self, other: Self) -> Self {
            let product = u16::from(self.0) * u16::from(other.0);
            Self(u8::try_from(product / 100).expect("volume within 0..=100"))
        }
    }

    /// represents panning value in range -100 to 100
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Panning(i8);

    impl Default for Panning {
        fn default() -> Self {
            Self::CENTER
        }
    }

    impl Panning {
        /// Centered panning (0).
        pub const CENTER: Self = Self(0);

        pub fn new(panning: i8) -> Result<Self> {
            match &panning {
                (-100..=100) => Ok(Self(panning)),
                _ => Err(Error::InvalidPanning(panning.to_string())),
            }
        }

        pub const fn get(&self) -> i8 {
            self.0
        }

        /// Combines a note's panning with its layer's, as NBS playback does.
        /// A centered layer leaves the note's panning unchanged.
        pub const fn combine(self, layer: Self) -> Self {
            match layer.0 {
                0 => self,
                layer => Self(self.0.midpoint(layer)),
            }
        }
    }

    /// Represents the layer index
    pub type Index = u32;
    /// Represents the time step
    pub type Tick = u32;

    // error
    //
    // ++++++++++++============++++++++++++============++++++++++++============

    /// custom error types
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        // #[error("Invalid key: {0}")]
        // InvalidKey(String),
        #[error("Invalid or unsupported version: {0}")]
        InvalidVersion(String),
        #[error("Invalid panning value: {0} (must be -100 to 100)")]
        InvalidPanning(String),
        #[error("Invalid velocity value: {0} (must be 0-100)")]
        InvalidVolume(String),
        #[error(transparent)]
        Io(#[from] std::io::Error),
    }

    pub type Result<T> = std::result::Result<T, Error>;
}
