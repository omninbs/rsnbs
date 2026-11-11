//! leaf types: versions, anchors, positions, volumes, panning, and errors.

/// represents a valid nbs version format
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version(u8);

impl Version {
    /// The latest NBS file format version supported by this library.
    const LATEST: u8 = 6;

    /// creates a version, rejecting anything newer than the latest
    /// supported one.
    pub fn new(version: u8) -> Result<Self> {
        match version {
            0..=Self::LATEST => Ok(Self(version)),
            _ => Err(Error::InvalidVersion(version.to_string())),
        }
    }

    /// returns the raw version number.
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
    /// returns the tick coordinate.
    fn into_tick(self) -> Tick;
    /// returns a copy with the tick coordinate replaced.
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
    /// returns the layer index.
    fn into_layer(self) -> Index;
    /// returns a copy with the layer index replaced.
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
    /// creates a position from a tick and a layer index.
    pub const fn new(tick: Tick, layer: Index) -> Self {
        Self { tick, layer }
    }
}

// basic
//
// ++++++++++++============++++++++++++============++++++++++++============

/// represents a linear amplitude gain in range 0-100.
///
/// NBS stores this as a percentage, and OpenNBS applies it as a linear
/// amplitude multiplier: `100` is unity gain, `50` is half amplitude
/// (about `-6.02 dB`), and `0` is silence. It is not perceived loudness,
/// a dB value, or distance attenuation.
///
/// Note and layer volumes combine by multiplication, which is equivalent
/// to adding their dB values. See the [NBS specification](https://opennbs.org/nbs)
/// and OpenNBS's [playback implementation](https://github.com/OpenNBS/NoteBlockStudio)
/// (`scripts/play_sound/play_sound.gml`).
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

    /// creates a volume, rejecting values above `100`.
    pub fn new(volume: u8) -> Result<Self> {
        match volume {
            0..=100 => Ok(Self(volume)),
            _ => Err(Error::InvalidVolume(volume.to_string())),
        }
    }

    /// returns the raw gain in `0..=100`.
    pub const fn get(&self) -> u8 {
        self.0
    }

    /// Combines a note's volume with its layer's, as NBS playback does.
    ///
    /// Both operands are linear amplitude gains in `0..=100`, so the
    /// result is their product divided by 100. This is equivalent to
    /// adding their dB values, not averaging or adding percentages.
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

    /// creates a panning value, rejecting values outside `-100..=100`.
    pub fn new(panning: i8) -> Result<Self> {
        match &panning {
            (-100..=100) => Ok(Self(panning)),
            _ => Err(Error::InvalidPanning(panning.to_string())),
        }
    }

    /// returns the raw panning value.
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

/// result type for fallible nbs operations.
pub type Result<T> = std::result::Result<T, Error>;
