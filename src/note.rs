use crate::types::{Index, LayerAnchor, Panning, Position, Tick, TimeAnchor, Volume};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::num::NonZero;
use std::ops::{Deref, DerefMut};

// notes collection
//
// ++++++++++++============++++++++++++============++++++++++++============

/// ordered set of events, each identified by a unique anchor.
///
/// Keeps entries sorted by ascending anchor for deterministic iteration.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Notes<Anchor = Position, Event = Note>(BTreeMap<Anchor, Event>);

impl<A, E> Default for Notes<A, E> {
    fn default() -> Self {
        Notes(Default::default())
    }
}

impl<A, E> From<BTreeMap<A, E>> for Notes<A, E> {
    fn from(map: BTreeMap<A, E>) -> Self {
        Notes(map)
    }
}

impl<A: Ord, E> FromIterator<(A, E)> for Notes<A, E> {
    fn from_iter<I: IntoIterator<Item = (A, E)>>(iter: I) -> Self {
        Notes(BTreeMap::from_iter(iter))
    }
}

impl<A, E> Deref for Notes<A, E> {
    type Target = BTreeMap<A, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A, E> DerefMut for Notes<A, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<A, E> IntoIterator for Notes<A, E> {
    type Item = (A, E);
    type IntoIter = std::collections::btree_map::IntoIter<A, E>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, A, E> IntoIterator for &'a Notes<A, E> {
    type Item = (&'a A, &'a E);
    type IntoIter = std::collections::btree_map::Iter<'a, A, E>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// notes util
//
// ++++++++++++============++++++++++++============++++++++++++============

impl<A: TimeAnchor, E> Notes<A, E> {
    /// Rescales ticks from arbitrary tempo (tick/s) to standard game tick (20 t/s).
    ///
    /// Scaling down may drop anchor uniqueness; see
    /// [`rescale_to_tick_rate`](Self::rescale_to_tick_rate).
    pub fn rescale_to_game_tick(self, tempo: f32) -> impl Iterator<Item = (A, E)> {
        self.rescale_to_tick_rate(tempo, 20)
    }

    /// Rescales ticks from arbitrary tempo (tick/s) to redstone tick (10 t/s).
    ///
    /// Scaling down may drop anchor uniqueness; see
    /// [`rescale_to_tick_rate`](Self::rescale_to_tick_rate).
    pub fn rescale_to_redstone_tick(self, tempo: f32) -> impl Iterator<Item = (A, E)> {
        self.rescale_to_tick_rate(tempo, 10)
    }

    /// Rescales ticks from arbitrary tempo (tick/s) to the given target tick rate (t/s).
    ///
    /// The transform stays integral by approximating the factor to `{n, 1/n}`,
    /// and yields a plain `(A, E)` iterator rather than a [`Notes`] map so the
    /// caller can resolve collisions (e.g. via [`pack_layers`](Self::pack_layers)).
    ///
    /// Scaling down (factor < 1) may collapse several ticks onto one, so distinct
    /// anchors are not guaranteed to stay unique.
    pub fn rescale_to_tick_rate(
        self,
        tempo: f32,
        target_rate: u32,
    ) -> impl Iterator<Item = (A, E)> {
        // tempo outside (0, 30): assume NBS tick == game tick, fold by target/20
        let scale = match (0.0..30.0).contains(&tempo) {
            true => target_rate as f32 / tempo,
            false => target_rate as f32 / 20.0,
        };
        // approximate scale to {z, 1/z} as (num, den), keeping tick transforms integral
        let (num, den) = match scale >= 1.0 {
            true => (scale.round() as u32, 1),
            false => (1, (1.0 / scale).round() as u32),
        };
        self.into_iter().map(move |(anchor, event)| {
            let tick = anchor.into_tick() * num / den;
            (anchor.with_tick(tick), event)
        })
    }
}

impl<A, E> Notes<A, E>
where
    A: TimeAnchor + LayerAnchor + Default,
{
    /// Packs a time-anchored stream into this anchor type, assigning
    /// successive layers to same-tick events so none is lost.
    pub fn pack_layers<S, I>(notes: I) -> impl Iterator<Item = (A, E)>
    where
        S: TimeAnchor,
        I: IntoIterator<Item = (S, E)>,
    {
        let mut layers: BTreeMap<Tick, Index> = BTreeMap::new();
        notes.into_iter().map(move |(anchor, event)| {
            let tick = anchor.into_tick();
            let layer = layers.entry(tick).or_default();
            let pos = A::default().with_tick(tick).with_layer(*layer);
            *layer += 1;
            (pos, event)
        })
    }
}

impl<A: LayerAnchor + Ord, E> Notes<A, E> {
    /// Groups notes into contiguous blocks separated by empty layers.
    pub fn split_by_layer_gaps(self) -> Vec<Notes<A, E>> {
        let layers: BTreeSet<Index> = self.keys().map(|pos| pos.into_layer()).collect();
        let mut starts = Vec::new();
        let mut prev: Option<Index> = None;
        for layer in layers {
            starts.extend(prev.is_none_or(|p| p + 1 != layer).then_some(layer));
            prev = Some(layer);
        }

        let mut groups: Vec<Notes<A, E>> = Vec::new();
        groups.resize_with(starts.len(), Notes::default);
        for (pos, note) in self {
            let layer = pos.into_layer();
            let group = starts.partition_point(|&s| s <= layer) - 1;
            groups[group].insert(pos.with_layer(layer - starts[group]), note);
        }
        groups
    }

    /// Splits notes into groups of `size` layers each.
    pub fn split_by_layer_count(self, size: Option<NonZero<usize>>) -> Vec<Notes<A, E>> {
        let Some(size) = size.map(|s| s.get() as Index) else {
            return vec![self];
        };
        let mut groups: BTreeMap<Index, BTreeMap<A, E>> = BTreeMap::new();
        for (pos, note) in self {
            let layer = pos.into_layer();
            groups
                .entry(layer / size)
                .or_default()
                .insert(pos.with_layer(layer % size), note);
        }
        groups.into_values().map(Notes::from).collect()
    }

    /// Stacks note groups vertically with 2 blank layers between, consuming them by value.
    pub fn concat<I: IntoIterator<Item = Notes<A, E>>>(notes: I) -> impl Iterator<Item = (A, E)> {
        let stacked = notes.into_iter().scan(0, |offset, n| {
            let base = *offset;
            let f = move |(pos, note): (A, E)| (pos.with_layer(pos.into_layer() + base), note);
            *offset += n.keys().map(|p| p.into_layer()).max().map_or(0, |m| m + 2);
            Some(n.into_iter().map(f))
        });
        stacked.flatten()
    }
}

// note
//
// ++++++++++++============++++++++++++============++++++++++++============

/// a single note with timing, instrument, and modulation data.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    pub tone: Tone,
    pub velocity: Volume,
    pub panning: Panning,
    pub pitch: i16,
}

impl Note {
    /// creates a note from any value that can convert into one.
    pub fn new<T: Into<Self>>(value: T) -> Self {
        value.into()
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
    pub instrument: Instrument,
    pub key: Key,
}

impl Tone {
    pub fn new(instrument: Instrument, key: Key) -> Self {
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
    Harp,
    DoubleBass,
    BassDrum,
    SnareDrum,
    Click,
    Guitar,
    Flute,
    Bell,
    Chime,
    Xylophone,
    IronXylophone,
    CowBell,
    Didgeridoo,
    Bit,
    Banjo,
    Pling,
    Trumpet,
    TrumpetExposed,
    TrumpetWeathered,
    TrumpetOxidized,
    // Mob head instruments. Reserved by rsnbs: not part of the NBS file
    // format, used for schematic rendering of mob head note blocks. They
    // are not encodable in files and fall back to Harp when saved.
    Imitate(ImitateInstrument),
    // Custom instruments, identified by their slot index in the song's
    // custom instrument list (the NBS instrument byte minus the song's
    // first custom instrument index).
    Custom(u8),
}

/// mob head sounds, reserved by rsnbs for schematic rendering.
/// these are not NBS instruments and cannot be saved to files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImitateInstrument {
    Creeper,
    Skeleton,
    Dragon,
    WitherSkeleton,
    Piglin,
    Zombie,
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
    pub(crate) fn vanilla_count() -> u8 {
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
    pub fn nbs_definition(self) -> Option<(&'static str, &'static str)> {
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
        $( pub const $name: Key = Key($value); )*
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

    pub fn new(key: u8) -> Self {
        Self(key)
    }

    /// converts a minecraft note (0-24, f#3-f#5) to the corresponding nbs key.
    pub fn from_minecraft_note<T: TryInto<u8>>(note: T) -> Option<Self> {
        let key = note.try_into().ok()?.checked_add(33)?;
        if key <= 57 { Some(Self(key)) } else { None }
    }

    /// converts the nbs key to the corresponding minecraft note (0-24, f#3-f#5).
    pub fn minecraft_note(&self) -> Option<u8> {
        self.0.checked_sub(33).filter(|&n| n <= 24)
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
