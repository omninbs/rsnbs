//! The musical content of a score as this format stores it: positions, notes,
//! levels, layers, and custom sounds.

use crate::axiom::event::{Index, LayerAnchor, Tick, TimeAnchor};
use crate::game::minecraft::{Instrument, Key};
use crate::{Error, Result};

// position
//
// ++++++++++++============++++++++++++============++++++++++++============

/// The place an event occupies: a moment in time and a layer.
///
/// A position is unique within a score, so at most one event can sit on it.
///
/// Both coordinates are counted in this format's units, so a position from one
/// score does not address another.
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

impl TimeAnchor for Position {
    fn into_tick(self) -> Tick {
        self.tick
    }
    fn with_tick(self, tick: Tick) -> Self {
        Self { tick, ..self }
    }
}

impl LayerAnchor for Position {
    fn into_layer(self) -> Index {
        self.layer
    }
    fn with_layer(self, layer: Index) -> Self {
        Self { layer, ..self }
    }
}

// loudness and placement
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A linear amplitude gain in range 0-100.
///
/// Expresses how loud an event sounds.
///
/// The value is a percentage of unity gain: `0` is silence and `100` is full
/// amplitude. Gains combine by multiplication, never by addition.
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

    /// Combines a gain with a second one applied on top of it.
    ///
    /// The result is the product of the two gains, so combining with full
    /// volume leaves the gain unchanged and combining with silence silences it.
    pub fn combine(self, other: Self) -> Self {
        let product = u16::from(self.0) * u16::from(other.0);
        Self(u8::try_from(product / 100).expect("volume within 0..=100"))
    }
}

/// A position on the stereo field, from hard left to hard right.
///
/// Places a sound between the two channels.
///
/// The value is balanced around center: `-100` is hard left, `0` is center,
/// and `100` is hard right. Combining stays inside that range, and combining
/// with center leaves the value unchanged.
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

    /// Combines a position with a second one applied on top of it.
    ///
    /// A centered second value leaves the position unchanged; otherwise the
    /// result lies between the two, clamped to the valid range.
    pub const fn combine(self, layer: Self) -> Self {
        match layer.0 {
            0 => self,
            layer => Self(self.0.midpoint(layer)),
        }
    }
}

// note
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A single sounding event.
///
/// Describes what sounds, leaving when and where it sounds to its anchor.
///
/// Its fields are independent: pitch, loudness, and placement can each vary
/// without affecting the others.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    /// the sound source and the pitch it plays.
    pub tone: Tone,
    /// loudness of this event on its own; see [`Volume`].
    pub velocity: Volume,
    /// placement of this event on its own; see [`Panning`].
    pub panning: Panning,
    /// fine tuning away from [`Tone::key`].
    ///
    /// Zero means the key sounds exactly; positive and negative values shift it
    /// up and down, and the magnitude is a fraction of a semitone.
    pub pitch: i16,
}

impl Note {
    /// creates a note from any value that can convert into one.
    pub fn new<T: Into<Self>>(value: T) -> Self {
        value.into()
    }

    /// Resolves this note against the layer it sits on.
    ///
    /// Yields the note as it actually sounds: its own loudness and placement
    /// combined with the layer's. The result is lossy, since it has to fit back
    /// into a plain [`Note`].
    pub fn apply_layer(&self, layer: &Layer) -> Self {
        Self {
            tone: self.tone,
            velocity: self.velocity.combine(layer.volume),
            panning: self.panning.combine(layer.panning),
            pitch: self.pitch,
        }
    }
}

impl AsRef<Tone> for Note {
    fn as_ref(&self) -> &Tone {
        &self.tone
    }
}

impl From<Tone> for Note {
    fn from(tone: Tone) -> Self {
        Self {
            tone,
            ..Default::default()
        }
    }
}

impl From<&Tone> for Note {
    fn from(tone: &Tone) -> Self {
        (*tone).into()
    }
}

/// The sound a note makes: a sound source and a pitch on it.
///
/// Separates the timbre from the pitch, so the same melody can be replayed on
/// another instrument.
///
/// The key is interpreted within the playable range of the instrument.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tone {
    /// instrument that produces the sound.
    pub instrument: Instrument,
    /// musical key of the tone.
    pub key: Key,
}

impl Tone {
    /// creates a tone from an instrument and a key.
    pub const fn new(instrument: Instrument, key: Key) -> Self {
        Self { instrument, key }
    }
}

impl From<Note> for Tone {
    fn from(note: Note) -> Self {
        note.tone
    }
}

impl AsRef<Tone> for Tone {
    fn as_ref(&self) -> &Tone {
        self
    }
}

// layer
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A named group of events that share settings.
///
/// Lets a score adjust many events at once, and switch them off as a group.
///
/// A layer's settings apply on top of each event's own; a switched-off layer
/// removes its events from playback entirely.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Layer {
    /// display name of the layer.
    pub name: String,
    /// whether the layer is switched off.
    pub disabled: bool,
    /// loudness applied to every event on the layer.
    pub volume: Volume,
    /// placement applied to every event on the layer.
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

/// A sound the score defines itself rather than taking from the built-in set.
///
/// Lets a score carry its own samples and still name them by position.
///
/// A tone refers to it by slot in the score's table; `pitch` is the key the
/// sample sounds at when played unaltered.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomInstrument {
    /// display name of the sound.
    pub name: String,
    /// location of the sample.
    pub file: String,
    /// the key this sample is tuned to.
    pub pitch: u8,
    /// whether a preview should highlight the matching key while the score
    /// plays.
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

// stream util
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Adapters for streams of notes.
///
/// Every method is available on any iterator whose items pair an anchor with a
/// note, so an [`EventMap`](crate::axiom::event::EventMap) of notes qualifies.
pub trait NoteStreamExt: Sized {
    /// Resolves each note against the layer it sits on.
    ///
    /// Notes whose layer is switched off are dropped; an event on a layer the
    /// score does not declare is resolved as if its layer were neutral.
    fn apply_layers<A>(self, layers: impl AsRef<[Layer]>) -> impl Iterator<Item = (A, Note)>
    where
        Self: IntoIterator<Item = (A, Note)>,
        A: LayerAnchor,
    {
        static NEUTRAL: Layer = Layer::new();
        self.into_iter().filter_map(move |(anchor, note)| {
            let index = usize::try_from(anchor.into_layer()).unwrap_or(usize::MAX);
            let layer = layers.as_ref().get(index).unwrap_or(&NEUTRAL);
            (!layer.disabled).then(|| (anchor, note.apply_layer(layer)))
        })
    }
}

impl<T> NoteStreamExt for T {}
