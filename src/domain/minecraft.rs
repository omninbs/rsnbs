//! Minecraft note block rules: instrument catalogue, key layout, and tick rates.

use std::fmt::{self, Display, Formatter};

// instrument
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A sound source this game ships with.
///
/// Names a timbre in the game's catalogue, so an event can say what it sounds
/// like without describing the sound itself.
///
/// Built-in variants are stable identities. A custom instrument is referred to
/// by its slot in the song's own table, and an imitation marks a game sound
/// that has no musical pitch.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Instrument {
    /// harp (plucked string); the default instrument.
    #[default]
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
    /// trumpet.
    Trumpet,
    /// exposed copper trumpet.
    TrumpetExposed,
    /// weathered copper trumpet.
    TrumpetWeathered,
    /// oxidized copper trumpet.
    TrumpetOxidized,
    /// a game sound used as a one-shot effect.
    Imitate(ImitateInstrument),
    /// custom instrument at the given slot in the song's custom sound table.
    Custom(u8),
}

/// Game sounds that are not musical instruments.
///
/// Lets a song reuse a game effect for its own sake.
///
/// These sounds have no musical pitch, so a score can only refer to them; it
/// cannot store them.
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

impl Display for Instrument {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

// key
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A pitch, named by its position on the game's keyboard.
///
/// Names the pitch an event sounds at, independent of the instrument that plays
/// it.
///
/// The value counts keys from the bottom of the playable range; `33` is `F#3`.
/// Keys outside the game's range still name a pitch but cannot be played.
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
            #[doc = concat!("the `", stringify!($name), "` key (game value ", stringify!($value), ").")]
            pub const $name: Key = Key($value);
        )*
    };
}

impl Key {
    keys! {
        FS3 = 33, G3 = 34, GS3 = 35, A3 = 36, AS3 = 37, B3 = 38,
        C4 = 39, CS4 = 40, D4 = 41, DS4 = 42, E4 = 43, F4 = 44, FS4 = 45,
        G4 = 46, GS4 = 47, A4 = 48, AS4 = 49, B4 = 50,
        C5 = 51, CS5 = 52, D5 = 53, DS5 = 54, E5 = 55, F5 = 56, FS5 = 57,
    }

    /// creates a key from its raw game value (`33` is `F#3`).
    pub const fn new(key: u8) -> Self {
        Self(key)
    }

    /// returns the raw key value.
    pub const fn get(&self) -> u8 {
        self.0
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

impl Key {
    /// Converts a game note number into the key that plays it.
    ///
    /// The game counts notes from the bottom of its keyboard; anything outside
    /// the playable range has no key and yields `None`.
    pub fn from_minecraft_note<T: TryInto<u8>>(note: T) -> Option<Self> {
        let key = note.try_into().ok()?.checked_add(33)?;
        if key <= 57 {
            Some(Self::new(key))
        } else {
            None
        }
    }

    /// Converts the key into the game note that plays it.
    ///
    /// Keys outside the game's playable range yield `None`.
    pub const fn minecraft_note(&self) -> Option<u8> {
        match self.get().checked_sub(33) {
            Some(note) if note <= 24 => Some(note),
            _ => None,
        }
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const NOTE_NAMES: &[&str] = &[
            "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
        ];
        let raw = self.get();
        let note = NOTE_NAMES[(raw % 12) as usize];
        let octave = raw / 12;
        let clicks = self
            .minecraft_note()
            .map(|k| format!("{k:02} clicks"))
            .unwrap_or("invalid".into());
        write!(f, "{note}{octave} ({clicks})")
    }
}

// tick rate
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Ticks per second at which the game advances sound.
///
/// Feeds [`rescale_to_tick_rate`](crate::axiom::event::EventStreamExt::rescale_to_tick_rate)
/// when events must land on this game's own grid.
pub const TICK_RATE: u32 = 20;

/// Ticks per second at which the game's redstone circuitry advances.
pub const REDSTONE_TICK_RATE: u32 = 10;
