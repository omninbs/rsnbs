use crate::song::Layer;
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

// stream util
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Adapters for note streams: applies to anything iterable over
/// `(anchor, event)` pairs, including [`Notes`] itself.
pub trait NoteStreamExt: Sized {
    /// Rescales ticks from arbitrary tempo (tick/s) to standard game tick (20 t/s).
    ///
    /// Scaling down may drop anchor uniqueness; see
    /// [`rescale_to_tick_rate`](Self::rescale_to_tick_rate).
    fn rescale_to_game_tick<A, E>(self, tempo: f32) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
    {
        self.rescale_to_tick_rate(tempo, 20)
    }

    /// Rescales ticks from arbitrary tempo (tick/s) to redstone tick (10 t/s).
    ///
    /// Scaling down may drop anchor uniqueness; see
    /// [`rescale_to_tick_rate`](Self::rescale_to_tick_rate).
    fn rescale_to_redstone_tick<A, E>(self, tempo: f32) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
    {
        self.rescale_to_tick_rate(tempo, 10)
    }

    /// Rescales ticks from arbitrary tempo (tick/s) to the given target tick rate (t/s).
    ///
    /// The transform stays integral by approximating the factor to `{n, 1/n}`,
    /// and yields a plain `(A, E)` iterator rather than a [`Notes`] map so the
    /// caller can resolve collisions (e.g. via [`pack_layers`](Self::pack_layers)).
    ///
    /// Scaling down (factor < 1) may collapse several ticks onto one, so anchors
    /// are no longer guaranteed unique; an anchor-keyed collection then overwrites.
    fn rescale_to_tick_rate<A, E>(
        self,
        tempo: f32,
        target_rate: u32,
    ) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
    {
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

    /// Packs the stream into anchor `A2`, assigning successive layers to
    /// same-tick events so none is lost.
    fn pack_layers<A, E, A2>(self) -> impl Iterator<Item = (A2, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
        A2: TimeAnchor + LayerAnchor + Default,
    {
        let mut layers: BTreeMap<Tick, Index> = BTreeMap::new();
        self.into_iter().map(move |(anchor, event)| {
            let tick = anchor.into_tick();
            let layer = layers.entry(tick).or_default();
            let pos = A2::default().with_tick(tick).with_layer(*layer);
            *layer += 1;
            (pos, event)
        })
    }

    /// Resolves each note against the layer it sits on; see [`Note::apply_layer`].
    /// The result reflects how the notes actually sound: disabled layers
    /// contribute nothing, and layers the song doesn't define act as neutral.
    fn apply_layers<A, L>(self, layers: L) -> impl Iterator<Item = (A, Note)>
    where
        Self: IntoIterator<Item = (A, Note)>,
        A: LayerAnchor,
        L: AsRef<[Layer]>,
    {
        static FALLBACK: Layer = Layer::new();
        self.into_iter().filter_map(move |(anchor, note)| {
            let index = usize::try_from(anchor.into_layer()).unwrap_or(usize::MAX);
            let layer = layers.as_ref().get(index).unwrap_or(&FALLBACK);
            (!layer.disabled).then(|| (anchor, note.apply_layer(layer)))
        })
    }

    /// Inverse of the `split_by_*` methods: recombines several note groups
    /// into one, stacking them vertically in order so their anchors never
    /// collide. Consumes the groups by value and yields a flat event stream.
    fn concat<A, E>(self) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = Notes<A, E>>,
        A: LayerAnchor,
    {
        let stacked = self.into_iter().scan(0, |offset, n| {
            let base = *offset;
            let f = move |(pos, note): (A, E)| (pos.with_layer(pos.into_layer() + base), note);
            *offset += n.keys().map(|p| p.into_layer()).max().map_or(0, |m| m + 2);
            Some(n.into_iter().map(f))
        });
        stacked.flatten()
    }
}

/// Applies to every type; each method activates when its `Self` bounds hold.
impl<T> NoteStreamExt for T {}

// notes util
//
// ++++++++++++============++++++++++++============++++++++++++============

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
            let entry = groups.entry(layer / size).or_default();
            entry.insert(pos.with_layer(layer % size), note);
        }
        groups.into_values().map(Notes::from).collect()
    }
}

impl<A: TimeAnchor, E> Notes<A, E> {
    /// The number of ticks the collection spans from origin: one past the last
    /// anchor's tick, or 0 when empty.
    pub fn tick_count(&self) -> Tick {
        self.keys()
            .map(|anchor| anchor.into_tick())
            .max()
            .map_or(0, |tick| tick + 1)
    }
}

impl<A: LayerAnchor, E> Notes<A, E> {
    /// The number of layers the collection spans: one past the highest anchor
    /// layer, or 0 when empty.
    pub fn layer_count(&self) -> Index {
        self.keys()
            .map(|anchor| anchor.into_layer())
            .max()
            .map_or(0, |layer| layer + 1)
    }
}

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
