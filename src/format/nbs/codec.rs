//! Reading and writing: the codec traits, their hooks, and a score's entry
//! points.

use super::model::{CustomInstrument, Layer, Note};
use super::{NbsHeader, NbsSong, Version};
use crate::Result;
use crate::axiom::event::{Index, Tick, TimeAnchor};
use crate::game::minecraft::Instrument;
use std::borrow::Cow;
use std::io;

type CowHeader<'a> = Cow<'a, NbsHeader>;
type CowNote<'a> = Cow<'a, Note>;
type CowLayer<'a> = Cow<'a, Layer>;
type CowCustomInsts<'a> = Cow<'a, [CustomInstrument]>;
type CowCustomInst<'a> = Cow<'a, CustomInstrument>;

// codec
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Reading and writing for one type, optionally with shared context.
///
/// The context is the same in both directions, so a reader and a writer always
/// agree on what surrounds a value. Types that need nothing use `()`.
pub(super) trait Codec: Clone {
    /// what both directions need to know about the surroundings.
    type Context: Copy;

    /// reads one value from a reader.
    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<Self>;

    /// writes one value to a writer.
    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<()>;
}

// song
//
// ++++++++++++============++++++++++++============++++++++++++============

impl<E> NbsSong<E>
where
    E: Clone + From<Note> + Into<Note>,
{
    /// Reads an nbs file into a score.
    ///
    /// The returned score holds events of `E`; a reader must be able to obtain
    /// a note from each decoded event.
    pub fn open_nbs<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let mut file = std::fs::File::open(path)?;
        Self::parse(&mut file)
    }

    /// Reads an nbs file from standard input into a score.
    pub fn from_nbs_stdin() -> Result<Self> {
        let mut stdin = std::io::stdin();
        Self::parse(&mut stdin)
    }

    /// Reads a score from a reader.
    pub fn parse<R: io::Read>(reader: &mut R) -> Result<Self> {
        let mut hooks = PresetFold;
        Codec::parse(reader, (), &mut hooks)
    }

    /// Writes the score to an nbs file.
    pub fn save_nbs<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.write(&mut file)
    }

    /// Writes the score to standard output as an nbs file.
    pub fn to_nbs_stdout(&mut self) -> Result<()> {
        let mut stdout = std::io::stdout();
        self.write(&mut stdout)
    }

    /// Writes the score to a writer.
    pub fn write<W: io::Write>(&self, writer: &mut W) -> Result<()> {
        let mut hooks = (PresetInject::new(), HeaderStats::new());
        Codec::write(self, writer, (), &mut hooks)
    }
}

// transformer
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Hooks that observe one level of a score while it is read or written.
///
/// Reading passes decoded values through the `decode_*` hooks and writing
/// passes values through the `encode_*` hooks. The default implementation
/// returns every value unchanged, and a hook only overrides what it needs.
pub(super) trait Transformer {
    fn decode_header(&mut self, header: NbsHeader) -> NbsHeader {
        header
    }
    fn encode_header<'a>(&mut self, header: CowHeader<'a>) -> CowHeader<'a> {
        header
    }

    /// rewrites a decoded score.
    ///
    /// `E` must be able to carry an instrument across the conversion, as preset
    /// folding rewrites the instruments its events reference.
    fn decode_song<E>(&mut self, song: NbsSong<E>) -> NbsSong<E>
    where
        E: From<Note> + Into<Note>,
    {
        song
    }
    /// observes a score before it is written; events may be of any type.
    fn encode_song<E>(&mut self, _song: &NbsSong<E>) {}

    fn decode_note(&mut self, note: Note) -> Note {
        note
    }
    fn encode_note<'a>(&mut self, note: CowNote<'a>) -> CowNote<'a> {
        note
    }

    fn decode_layer(&mut self, layer: Layer) -> Layer {
        layer
    }
    fn encode_layer<'a>(&mut self, layer: CowLayer<'a>) -> CowLayer<'a> {
        layer
    }

    fn decode_custom_insts(&mut self, customs: Vec<CustomInstrument>) -> Vec<CustomInstrument> {
        customs
    }
    fn encode_custom_insts<'a>(&mut self, customs: CowCustomInsts<'a>) -> CowCustomInsts<'a> {
        customs
    }

    fn decode_custom_inst(&mut self, instrument: CustomInstrument) -> CustomInstrument {
        instrument
    }
    fn encode_custom_inst<'a>(&mut self, instrument: CowCustomInst<'a>) -> CowCustomInst<'a> {
        instrument
    }

    fn decode_instrument(&mut self, instrument: Instrument) -> Instrument {
        instrument
    }
    fn encode_instrument(&mut self, instrument: Instrument) -> Instrument {
        instrument
    }
}

/// A chain with nothing to do.
impl Transformer for () {}

/// Chains two hooks; the first runs first.
impl<A: Transformer, B: Transformer> Transformer for (A, B) {
    fn decode_header(&mut self, header: NbsHeader) -> NbsHeader {
        let header = self.0.decode_header(header);
        self.1.decode_header(header)
    }

    fn encode_header<'a>(&mut self, header: CowHeader<'a>) -> CowHeader<'a> {
        let header = self.0.encode_header(header);
        self.1.encode_header(header)
    }

    fn decode_song<E>(&mut self, song: NbsSong<E>) -> NbsSong<E>
    where
        E: From<Note> + Into<Note>,
    {
        let song = self.0.decode_song(song);
        self.1.decode_song(song)
    }

    fn encode_song<E>(&mut self, song: &NbsSong<E>) {
        self.0.encode_song(song);
        self.1.encode_song(song);
    }

    fn decode_custom_insts(&mut self, customs: Vec<CustomInstrument>) -> Vec<CustomInstrument> {
        let customs = self.0.decode_custom_insts(customs);
        self.1.decode_custom_insts(customs)
    }

    fn encode_custom_insts<'a>(&mut self, customs: CowCustomInsts<'a>) -> CowCustomInsts<'a> {
        let customs = self.0.encode_custom_insts(customs);
        self.1.encode_custom_insts(customs)
    }

    fn decode_note(&mut self, note: Note) -> Note {
        let note = self.0.decode_note(note);
        self.1.decode_note(note)
    }

    fn encode_note<'a>(&mut self, note: CowNote<'a>) -> CowNote<'a> {
        let note = self.0.encode_note(note);
        self.1.encode_note(note)
    }

    fn decode_layer(&mut self, layer: Layer) -> Layer {
        let layer = self.0.decode_layer(layer);
        self.1.decode_layer(layer)
    }

    fn encode_layer<'a>(&mut self, layer: CowLayer<'a>) -> CowLayer<'a> {
        let layer = self.0.encode_layer(layer);
        self.1.encode_layer(layer)
    }

    fn decode_custom_inst(&mut self, instrument: CustomInstrument) -> CustomInstrument {
        let instrument = self.0.decode_custom_inst(instrument);
        self.1.decode_custom_inst(instrument)
    }

    fn encode_custom_inst<'a>(&mut self, instrument: CowCustomInst<'a>) -> CowCustomInst<'a> {
        let instrument = self.0.encode_custom_inst(instrument);
        self.1.encode_custom_inst(instrument)
    }

    fn decode_instrument(&mut self, instrument: Instrument) -> Instrument {
        let instrument = self.0.decode_instrument(instrument);
        self.1.decode_instrument(instrument)
    }

    fn encode_instrument(&mut self, instrument: Instrument) -> Instrument {
        let instrument = self.0.encode_instrument(instrument);
        self.1.encode_instrument(instrument)
    }
}

// hooks
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Reads back the preset entries [`PresetInject`] appends on write.
///
/// A preset entry returns to being a built-in instrument, and the custom table
/// is renumbered to close the gap; every reference follows its entry.
pub(super) struct PresetFold;

impl Transformer for PresetFold {
    /// Folds preset entries back into vanilla instruments, compressing the rest.
    fn decode_song<E>(&mut self, mut song: NbsSong<E>) -> NbsSong<E>
    where
        E: From<Note> + Into<Note>,
    {
        let fci = song.header.version.vanilla_instruments() as usize;
        let fold = |custom: &CustomInstrument| {
            let definition = Some((custom.name.as_str(), custom.file.as_str()));
            let mut vanilla = Instrument::NBS_INDEX.into_iter().skip(fci);
            vanilla.find(|instrument| instrument.nbs_definition() == definition)
        };

        // split: folded entries leave the table, kept entries take their slot
        let mut kept = Vec::new();
        let mut slots = Vec::new();
        for custom in std::mem::take(&mut song.custom_instruments) {
            let vanilla = fold(&custom);
            slots.push(vanilla.unwrap_or(Instrument::Custom(kept.len() as u8)));
            if vanilla.is_none() {
                kept.push(custom);
            }
        }
        song.custom_instruments = kept;
        let identity = |(i, slot): (usize, &Instrument)| *slot == Instrument::Custom(i as u8);
        if slots.iter().enumerate().all(identity) {
            return song;
        }

        // every reference follows its slot: vanilla, or the compressed one
        let remap = |instrument: Instrument| match instrument {
            Instrument::Custom(slot) => slots.get(slot as usize).copied().unwrap_or(instrument),
            other => other,
        };
        song.notes = std::mem::take(&mut song.notes)
            .into_iter()
            .map(|(pos, event)| {
                let mut note: Note = event.into();
                note.tone.instrument = remap(note.tone.instrument);
                (pos, note.into())
            })
            .collect();
        song
    }
}

/// Keeps built-in instruments addressable in a version that cannot index them.
///
/// Such an instrument is stored as a custom entry instead, and every reference
/// to it is rewritten to that entry. The table grows only as needed.
pub(super) struct PresetInject {
    version: Option<Version>,
    /// the table being built: original entries plus any presets appended for
    /// this write.
    customs: Vec<CustomInstrument>,
}

impl PresetInject {
    pub(super) fn new() -> Self {
        PresetInject {
            version: None,
            customs: Vec::new(),
        }
    }
}

impl Transformer for PresetInject {
    /// Records the version and takes over the custom table.
    fn encode_song<E>(&mut self, song: &NbsSong<E>) {
        self.version = Some(song.header.version);
        self.customs = song.custom_instruments.clone();
    }

    /// Returns the custom table, presets included.
    fn encode_custom_insts<'a>(&mut self, _: CowCustomInsts<'a>) -> CowCustomInsts<'a> {
        Cow::Owned(std::mem::take(&mut self.customs))
    }

    /// Points vanilla instruments above the FCI at a custom entry.
    fn encode_instrument(&mut self, instrument: Instrument) -> Instrument {
        let fci = self.version.unwrap().vanilla_instruments();
        if instrument.vanilla_index().is_none_or(|i| i < fci) {
            return instrument;
        }
        let (name, file) = instrument.nbs_definition().unwrap();
        if let Some(slot) = self
            .customs
            .iter()
            .position(|c| c.name == name && c.file == file)
        {
            return Instrument::Custom(slot as u8);
        }
        self.customs.push(CustomInstrument {
            name: name.to_owned(),
            file: file.to_owned(),
            ..Default::default()
        });
        Instrument::Custom((self.customs.len() - 1) as u8)
    }
}

/// Fills in the parts of the header that follow from the score itself.
///
/// Values the events already determine, such as the length, are derived rather
/// than trusted, so a written header agrees with what is written after it.
pub(super) struct HeaderStats {
    song_length: Option<Tick>,
    song_layers: Option<Index>,
}

impl HeaderStats {
    pub(super) fn new() -> Self {
        HeaderStats {
            song_length: None,
            song_layers: None,
        }
    }
}

impl Transformer for HeaderStats {
    fn encode_song<E>(&mut self, song: &NbsSong<E>) {
        let last = song.notes.last_key_value();
        self.song_length = Some(last.map(|(p, _)| p.into_tick()).unwrap_or(1));
        self.song_layers = Some(song.layers.len() as _);
    }
    fn encode_header<'a>(&mut self, header: CowHeader<'a>) -> CowHeader<'a> {
        let mut header = header.into_owned();
        header.song_length = self.song_length.unwrap();
        header.song_layers = self.song_layers.unwrap();
        header.default_instruments = header.version.vanilla_instruments();
        Cow::Owned(header)
    }
}

// instrument
//
// ++++++++++++============++++++++++++============++++++++++++============

impl Instrument {
    /// Vanilla instruments in NBS serialization order; the array position is
    /// the NBS instrument index. The four v6 trumpets occupy indexes 16-19.
    pub(super) const NBS_INDEX: [Instrument; 20] = [
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
    pub(super) const fn vanilla_count() -> u8 {
        Self::NBS_INDEX.len() as u8
    }

    /// Returns the fixed table index of a vanilla instrument, if this is one.
    /// Vanilla instruments occupy indexes 0..20 (including the v6 trumpets).
    pub(super) fn vanilla_index(self) -> Option<u8> {
        Self::NBS_INDEX
            .iter()
            .position(|&inst| inst == self)
            .map(|index| index as u8)
    }

    /// Returns the canonical OpenNBS name and sound file of a vanilla
    /// instrument, used as the generic fallback when such an instrument must
    /// be stored as a custom instrument in an older NBS version.
    pub(super) const fn nbs_definition(self) -> Option<(&'static str, &'static str)> {
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
