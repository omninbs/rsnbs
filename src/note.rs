//! notes, tones, instruments, and keys.

use crate::song::Layer;
use crate::types::{LayerAnchor, Panning, Volume};
use std::fmt::{self, Display, Formatter};

// stream util
//
// ++++++++++++============++++++++++++============++++++++++++============

pub use crate::event::EventStreamExt;

/// adapters for note streams.
///
/// Each method applies when the receiver yields anchored notes, so an
/// [`EventMap`](crate::event::EventMap) of notes qualifies.
pub trait NoteStreamExt: Sized {
    /// Resolves each note against the layer it sits on, dropping notes on
    /// disabled layers.
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

// note
//
// ++++++++++++============++++++++++++============++++++++++++============

/// a single note with timing, instrument, and modulation data.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    /// instrument and base key of the note.
    pub tone: Tone,
    /// linear amplitude gain in `0..=100`; see [`Volume`].
    pub velocity: Volume,
    /// stereo position in `-100..=100`: `-100` is hard left, `0` is center,
    /// and `100` is hard right; see [`Panning`].
    pub panning: Panning,
    /// fine pitch offset in cents, relative to [`Tone::key`]: `0` means no
    /// detuning, `±100` cents is one semitone, and Note Block Studio limits
    /// the stored `i16` to `-1200..=1200`.
    pub pitch: i16,
}

impl Note {
    /// creates a note from any value that can convert into one.
    pub fn new<T: Into<Self>>(value: T) -> Self {
        value.into()
    }

    /// Resolves this note against the layer it sits on, giving the note as it
    /// actually sounds rather than the raw on-disk value.
    ///
    /// The result is lossy, as it has to fit back into a plain [`Note`].
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
        tone.clone().into()
    }
}

/// a tone is a pair of an instrument and a key.
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

// instrument
//
// ++++++++++++============++++++++++++============++++++++++++============

/// built-in minecraft note block instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Instrument {
    /// harp (plucked string); the default instrument.
    Harp,
    /// double bass.
    DoubleBass,
    /// bass drum.
    BassDrum,
    /// snare drum.
    SnareDrum,
    /// click (hi-hat style).
    Click,
    /// guitar.
    Guitar,
    /// flute.
    Flute,
    /// bell.
    Bell,
    /// ice chime.
    Chime,
    /// xylophone.
    Xylophone,
    /// iron xylophone.
    IronXylophone,
    /// cow bell.
    CowBell,
    /// didgeridoo.
    Didgeridoo,
    /// bit (retro square wave).
    Bit,
    /// banjo.
    Banjo,
    /// pling (electric piano).
    Pling,
    /// trumpet, added in nbs version 6.
    Trumpet,
    /// exposed copper trumpet, added in nbs version 6.
    TrumpetExposed,
    /// weathered copper trumpet, added in nbs version 6.
    TrumpetWeathered,
    /// oxidized copper trumpet, added in nbs version 6.
    TrumpetOxidized,
    /// mob head imitation sound: reserved by rsnbs for schematic rendering,
    /// not part of the nbs format, and not encodable to files.
    Imitate(ImitateInstrument),
    /// custom instrument at the given slot in the song's custom instrument list.
    Custom(u8),
}

/// mob head sounds, reserved by rsnbs for schematic rendering.
/// these are not NBS instruments and cannot be saved to files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImitateInstrument {
    /// creeper hiss.
    Creeper,
    /// skeleton rattle.
    Skeleton,
    /// ender dragon growl.
    Dragon,
    /// wither skeleton rattle.
    WitherSkeleton,
    /// piglin grunt.
    Piglin,
    /// zombie groan.
    Zombie,
    /// sound of a custom player head.
    CustomHead,
}

impl Default for Instrument {
    fn default() -> Self {
        Self::Harp
    }
}

impl Instrument {
    /// Vanilla instruments in NBS serialization order; the array position is
    /// the NBS instrument index. The four v6 trumpets occupy indexes 16-19.
    /// Note: byte encoding/decoding relative to the first custom instrument
    /// index lives in codec.rs.
    pub(crate) const NBS_INDEX: [Instrument; 20] = [
        Instrument::Harp,
        Instrument::DoubleBass,
        Instrument::BassDrum,
        Instrument::SnareDrum,
        Instrument::Click,
        Instrument::Guitar,
        Instrument::Flute,
        Instrument::Bell,
        Instrument::Chime,
        Instrument::Xylophone,
        Instrument::IronXylophone,
        Instrument::CowBell,
        Instrument::Didgeridoo,
        Instrument::Bit,
        Instrument::Banjo,
        Instrument::Pling,
        Instrument::Trumpet,
        Instrument::TrumpetExposed,
        Instrument::TrumpetWeathered,
        Instrument::TrumpetOxidized,
    ];

    /// The number of vanilla instruments in the newest NBS version.
    pub(crate) const fn vanilla_count() -> u8 {
        Self::NBS_INDEX.len() as u8
    }

    /// Returns the fixed table index of a vanilla instrument, if this is one.
    /// Vanilla instruments occupy indexes 0..20 (including the v6 trumpets).
    pub fn vanilla_index(self) -> Option<u8> {
        Self::NBS_INDEX
            .iter()
            .position(|&inst| inst == self)
            .map(|index| index as u8)
    }

    /// Returns the canonical OpenNBS name and sound file of a vanilla
    /// instrument, used as the generic fallback when such an instrument must
    /// be stored as a custom instrument in an older NBS version.
    pub const fn nbs_definition(self) -> Option<(&'static str, &'static str)> {
        use Instrument::*;
        Some(match self {
            Harp => ("Harp", "harp.ogg"),
            DoubleBass => ("Double Bass", "dbass.ogg"),
            BassDrum => ("Bass Drum", "bdrum.ogg"),
            SnareDrum => ("Snare Drum", "sdrum.ogg"),
            Click => ("Click", "click.ogg"),
            Guitar => ("Guitar", "guitar.ogg"),
            Flute => ("Flute", "flute.ogg"),
            Bell => ("Bell", "bell.ogg"),
            Chime => ("Chime", "icechime.ogg"),
            Xylophone => ("Xylophone", "xylobone.ogg"),
            IronXylophone => ("Iron Xylophone", "iron_xylophone.ogg"),
            CowBell => ("Cow Bell", "cow_bell.ogg"),
            Didgeridoo => ("Didgeridoo", "didgeridoo.ogg"),
            Bit => ("Bit", "bit.ogg"),
            Banjo => ("Banjo", "banjo.ogg"),
            Pling => ("Pling", "pling.ogg"),
            Trumpet => ("Trumpet", "trumpet.ogg"),
            TrumpetExposed => ("Exposed Trumpet", "trumpet_exposed.ogg"),
            TrumpetWeathered => ("Weathered Trumpet", "trumpet_weathered.ogg"),
            TrumpetOxidized => ("Oxidized Trumpet", "trumpet_oxidized.ogg"),
            _ => return None,
        })
    }
}

impl Display for Instrument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

// key
//
// ++++++++++++============++++++++++++============++++++++++++============

/// a musical key (f#3-f#5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key(u8);

impl Default for Key {
    fn default() -> Self {
        Self::FS3
    }
}

macro_rules! keys {
    ($($name:ident = $value:expr),* $(,)?) => {
        $(
            #[doc = concat!("the `", stringify!($name), "` key (nbs value ", stringify!($value), ").")]
            pub const $name: Key = Key($value);
        )*
    };
}

impl Key {
    // F#3 = 33 = note(0)
    keys! {
        FS3 = 33, G3 = 34, GS3 = 35, A3 = 36, AS3 = 37, B3 = 38,
        C4 = 39, CS4 = 40, D4 = 41, DS4 = 42, E4 = 43, F4 = 44, FS4 = 45,
        G4 = 46, GS4 = 47, A4 = 48, AS4 = 49, B4 = 50,
        C5 = 51, CS5 = 52, D5 = 53, DS5 = 54, E5 = 55, F5 = 56, FS5 = 57,
    }

    /// creates a key from its raw nbs value (`33` is `F#3`).
    pub const fn new(key: u8) -> Self {
        Self(key)
    }

    /// converts a minecraft note (0-24, f#3-f#5) to the corresponding nbs key.
    pub fn from_minecraft_note<T: TryInto<u8>>(note: T) -> Option<Self> {
        let key = note.try_into().ok()?.checked_add(33)?;
        if key <= 57 { Some(Self(key)) } else { None }
    }

    /// converts the nbs key to the corresponding minecraft note (0-24, f#3-f#5).
    pub const fn minecraft_note(&self) -> Option<u8> {
        match self.0.checked_sub(33) {
            Some(note) if note <= 24 => Some(note),
            _ => None,
        }
    }
}

impl From<u8> for Key {
    fn from(value: u8) -> Self {
        Key(value)
    }
}

impl From<Key> for u8 {
    fn from(value: Key) -> Self {
        value.0
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const NOTE_NAMES: &[&str] = &[
            "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
        ];
        let note = NOTE_NAMES[(self.0 % 12) as usize];
        let octave = self.0 / 12;
        let clicks = self
            .minecraft_note()
            .map(|k| format!("{k:02} clicks"))
            .unwrap_or("invalid".into());
        write!(f, "{note}{octave} ({clicks})")
    }
}
