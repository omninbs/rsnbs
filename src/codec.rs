//! NBS (Note Block Studio) file format parser and writer.

use crate::note::{Instrument, Note, Notes};
use crate::song::{CustomInstrument, Header, Layer, Song};
use crate::types::{Index, Position, Result, Tick, TimeAnchor, Version};
use std::borrow::Cow;
use std::io;

mod impls;
mod nbs_ext;

type CowHeader<'a> = Cow<'a, Header>;
type CowNote<'a> = Cow<'a, Note>;
type CowLayer<'a> = Cow<'a, Layer>;
type CowCustomInsts<'a> = Cow<'a, [CustomInstrument]>;
type CowCustomInst<'a> = Cow<'a, CustomInstrument>;
type CowSong<'a> = Cow<'a, Song>;

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

impl Song {
    /// parses a complete Song from a reader
    pub fn parse<R: io::Read>(reader: &mut R) -> Result<Self> {
        let mut hooks = (InstrumentTranslate::new(), HeaderStats::new());
        Codec::parse(reader, (), &mut hooks)
    }

    /// writes the song to a writer.
    pub fn write<W: io::Write>(&self, writer: &mut W) -> Result<()> {
        let mut hooks = (InstrumentTranslate::new(), HeaderStats::new());
        Codec::write(self, writer, (), &mut hooks)
    }
}

// InstrumentTranslate
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Folds preset instruments on parse, injects them on write.
pub(super) struct InstrumentTranslate {
    version: Option<Version>,
    /// Vanilla instruments needing presets, collected lazily on write.
    used: Vec<Instrument>,
    /// Custom-table entries matching built-in definitions: instrument -> slot.
    reuse: Vec<(Instrument, u8)>,
    /// Custom table length; preset slots follow it.
    customs_len: usize,
}

impl InstrumentTranslate {
    pub(super) fn new() -> Self {
        InstrumentTranslate {
            version: None,
            used: Vec::new(),
            reuse: Vec::new(),
            customs_len: 0,
        }
    }

    /// The built-in instrument matching a custom entry beyond the FCI, if any.
    fn match_builtin(fci: usize, custom: &CustomInstrument) -> Option<Instrument> {
        Instrument::NBS_INDEX.into_iter().skip(fci).find(|inst| {
            inst.nbs_definition() == Some((custom.name.as_str(), custom.file.as_str()))
        })
    }

    /// (built-in, slot) for a custom entry matching a built-in, if any.
    fn reusable_slot(
        fci: usize,
        slot: usize,
        custom: &CustomInstrument,
    ) -> Option<(Instrument, u8)> {
        Self::match_builtin(fci, custom).map(|inst| (inst, slot as u8))
    }

    /// Reuses a matching custom entry, else lazily assigns a preset slot.
    fn slot_for(&mut self, inst: Instrument) -> u8 {
        let mut reuse = self.reuse.iter();
        if let Some(slot) = reuse.find_map(|&(i, s)| (i == inst).then_some(s)) {
            return slot;
        }
        if let Some(rank) = self.used.iter().position(|&u| u == inst) {
            return self.customs_len as u8 + rank as u8;
        }
        self.used.push(inst);
        self.customs_len as u8 + (self.used.len() - 1) as u8
    }

    /// Preset definitions for the collected vanilla instruments.
    fn preset_definitions(&self) -> impl Iterator<Item = CustomInstrument> {
        let custom = |name: &str, file: &str| CustomInstrument {
            name: name.into(),
            file: file.into(),
            pitch: 45,
            press_key: true,
        };
        self.used.iter().map(move |&instrument| {
            let (name, file) = instrument.nbs_definition().unwrap();
            custom(name, file)
        })
    }

    /// Folds preset entries back into vanilla instruments, compressing the rest.
    fn fold_preset_instruments(
        &self,
        mut notes: Notes<Position, Note>,
        custom_instruments: Vec<CustomInstrument>,
    ) -> (Notes<Position, Note>, Vec<CustomInstrument>) {
        let fci = self.version.unwrap().vanilla_instruments() as usize;
        let fold_entry = |(slot, custom): (usize, &CustomInstrument)| {
            Self::match_builtin(fci, custom).map(|vanilla| (slot as u8, vanilla))
        };
        let folded: Vec<(u8, Instrument)> = custom_instruments
            .iter()
            .enumerate()
            .filter_map(fold_entry)
            .collect();
        if folded.is_empty() {
            return (notes, custom_instruments);
        }

        let keep_entry = |(slot, custom): (usize, CustomInstrument)| {
            let keep = folded
                .binary_search_by_key(&(slot as u8), |&(s, _)| s)
                .is_err();
            keep.then_some(custom)
        };
        let kept: Vec<CustomInstrument> = custom_instruments
            .into_iter()
            .enumerate()
            .filter_map(keep_entry)
            .collect();

        for (_, note) in notes.iter_mut() {
            let Instrument::Custom(slot) = note.tone.instrument else {
                continue;
            };
            let instrument = folded
                .binary_search_by_key(&slot, |&(s, _)| s)
                .map(|index| folded[index].1)
                .unwrap_or_else(|insert| Instrument::Custom(slot - insert as u8));
            note.tone.instrument = instrument;
        }

        (notes, kept)
    }
}

impl Transformer for InstrumentTranslate {
    fn decode_header(&mut self, header: Header) -> Header {
        self.version = Some(header.version);
        header
    }

    /// Folds preset instruments into the song.
    fn decode_song(&mut self, mut song: Song) -> Song {
        (song.notes, song.custom_instruments) =
            self.fold_preset_instruments(song.notes, song.custom_instruments);
        song
    }

    /// Records the custom table length and built-in entries reusable on write.
    fn encode_song<'a>(&mut self, song: CowSong<'a>) -> CowSong<'a> {
        let version = song.header.version;
        self.version = Some(version);
        self.customs_len = song.custom_instruments.len();
        let fci = version.vanilla_instruments() as usize;
        let customs = song.custom_instruments.iter().enumerate();
        let reuse = customs.filter_map(|(slot, custom)| Self::reusable_slot(fci, slot, custom));
        self.reuse = reuse.collect();
        song
    }

    /// Appends the collected presets after the custom table.
    fn encode_custom_insts<'a>(&mut self, customs: CowCustomInsts<'a>) -> CowCustomInsts<'a> {
        if self.used.is_empty() {
            return customs;
        }
        let mut customs = customs.into_owned();
        customs.extend(self.preset_definitions());
        Cow::Owned(customs)
    }

    /// Maps vanilla instruments above the FCI onto the custom table.
    fn encode_instrument(&mut self, instrument: Instrument) -> Instrument {
        let fci = self.version.unwrap().vanilla_instruments();
        match instrument {
            Instrument::Custom(slot) => Instrument::Custom(slot),
            inst if inst.vanilla_index().map_or(true, |i| i < fci) => inst,
            inst => Instrument::Custom(self.slot_for(inst)),
        }
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
    fn encode_song<'a>(&mut self, song: CowSong<'a>) -> CowSong<'a> {
        let last = song.notes.last_key_value();
        self.song_length = Some(last.map(|(p, _)| p.into_tick()).unwrap_or(1));
        self.song_layers = Some(song.layers.len() as _);
        song
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

    fn decode_song(&mut self, song: Song) -> Song {
        song
    }
    fn encode_song<'a>(&mut self, song: CowSong<'a>) -> CowSong<'a> {
        song
    }

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

    fn decode_song(&mut self, song: Song) -> Song {
        let song = self.0.decode_song(song);
        self.1.decode_song(song)
    }

    fn encode_song<'a>(&mut self, song: CowSong<'a>) -> CowSong<'a> {
        let song = self.0.encode_song(song);
        self.1.encode_song(song)
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
