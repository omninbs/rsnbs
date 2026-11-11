//! NBS (Note Block Studio) file format parser and writer.

use crate::note::{Instrument, Note};
use crate::song::{CustomInstrument, Header, Layer, Song};
use crate::types::{Index, Result, Tick, TimeAnchor, Version};
use std::borrow::Cow;
use std::io;

mod ext;
mod impls;

type CowHeader<'a> = Cow<'a, Header>;
type CowNote<'a> = Cow<'a, Note>;
type CowLayer<'a> = Cow<'a, Layer>;
type CowCustomInsts<'a> = Cow<'a, [CustomInstrument]>;
type CowCustomInst<'a> = Cow<'a, CustomInstrument>;

// Parse/Write
//
// ++++++++++++============++++++++++++============++++++++++++============

/// unified trait for both parsing and writing data, optionally with context
pub(super) trait Codec: Clone {
    /// context type shared for both parsing and writing (use () when no context is needed)
    type Context: Copy;

    /// parse data from a reader with context
    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<Self>;

    /// write data to a writer with context
    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<()>;
}

// Song
//
// ++++++++++++============++++++++++++============++++++++++++============

impl<E> Song<E>
where
    E: Clone + From<Note> + Into<Note>,
{
    /// parses a complete Song from a reader
    pub fn parse<R: io::Read>(reader: &mut R) -> Result<Self> {
        let mut hooks = PresetFold;
        Codec::parse(reader, (), &mut hooks)
    }

    /// writes the song to a writer.
    pub fn write<W: io::Write>(&self, writer: &mut W) -> Result<()> {
        let mut hooks = (PresetInject::new(), HeaderStats::new());
        Codec::write(self, writer, (), &mut hooks)
    }
}

// PresetFold & PresetInject
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Reads back the preset entries [`PresetInject`] appends on write.
pub(super) struct PresetFold;

impl Transformer for PresetFold {
    /// Folds preset entries back into vanilla instruments, compressing the rest.
    fn decode_song<E>(&mut self, mut song: Song<E>) -> Song<E>
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

/// Appends preset entries for vanilla instruments the target version cannot
/// index.
pub(super) struct PresetInject {
    version: Option<Version>,
    /// Custom table being built: original entries plus injected presets, whose
    /// slots are their positions here.
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
    fn encode_song<E>(&mut self, song: &Song<E>) {
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

// HeaderStats
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Derives song statistics into the header on write.
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
    fn encode_song<E>(&mut self, song: &Song<E>) {
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

// Transformer
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Per-level transform hooks; parse calls decode_*, write calls encode_*.
pub(super) trait Transformer {
    fn decode_header(&mut self, header: Header) -> Header {
        header
    }
    fn encode_header<'a>(&mut self, header: CowHeader<'a>) -> CowHeader<'a> {
        header
    }

    /// transforms the song produced by decoding.
    ///
    /// `E` must round-trip the instrument its events reference, as preset
    /// folding rewrites it.
    fn decode_song<E>(&mut self, song: Song<E>) -> Song<E>
    where
        E: From<Note> + Into<Note>,
    {
        song
    }
    /// observes the song before writing; events may be of any type.
    fn encode_song<E>(&mut self, _song: &Song<E>) {}

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

/// Identity chain tail.
impl Transformer for () {}

/// Tuple combinator: chains two hooks, .0 runs first.
impl<A: Transformer, B: Transformer> Transformer for (A, B) {
    fn decode_header(&mut self, header: Header) -> Header {
        let header = self.0.decode_header(header);
        self.1.decode_header(header)
    }

    fn encode_header<'a>(&mut self, header: CowHeader<'a>) -> CowHeader<'a> {
        let header = self.0.encode_header(header);
        self.1.encode_header(header)
    }

    fn decode_song<E>(&mut self, song: Song<E>) -> Song<E>
    where
        E: From<Note> + Into<Note>,
    {
        let song = self.0.decode_song(song);
        self.1.decode_song(song)
    }

    fn encode_song<E>(&mut self, song: &Song<E>) {
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
