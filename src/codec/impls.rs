//! Concrete read/write logic: song skeleton, preset folding, per-type codecs.

use super::nbs_ext::{NbsReadExt, NbsWriteExt};
use super::{Codec, Transformer};
use crate::note::{Instrument, Key, Note, Notes, Tone};
use crate::song::{CustomInstrument, Header, Layer, Song};
use crate::types::{Index, LayerAnchor, Panning, Position, Result};
use crate::types::{Tick, TimeAnchor, Version, Volume};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io;
use std::num::NonZeroU32;

// Song
//
// ++++++++++++============++++++++++++============++++++++++++============

impl Codec for Song {
    type Context = ();

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        hooks: &mut M,
    ) -> Result<Self> {
        // 头部分
        let header = Header::parse(reader, (), hooks)?;
        let context = (header.version, header.default_instruments);

        // 音符部分
        let notes = Notes::parse(reader, context, hooks)?;

        // 层部分
        let mut layers = Vec::new();
        for _ in 0..header.song_layers {
            layers.push(Layer::parse(reader, header.version, hooks)?);
        }

        // 自定义乐器部分
        let instr_count = reader.read_u8()?;
        let mut custom_instruments = Vec::new();
        for _ in 0..instr_count {
            custom_instruments.push(CustomInstrument::parse(reader, (), hooks)?);
        }
        let custom_instruments = hooks.decode_custom_insts(custom_instruments);

        Ok(hooks.decode_song(Song {
            header,
            notes,
            layers,
            custom_instruments,
        }))
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        hooks: &mut M,
    ) -> Result<()> {
        let song = hooks.encode_song(Cow::Borrowed(self));
        let version = song.header.version;
        let fci = version.vanilla_instruments();
        let context = (version, fci);
        let song = song.as_ref();

        // 头部分
        song.header.write(writer, (), hooks)?;

        // 音符部分
        song.notes.write(writer, context, hooks)?;

        // 层部分
        for layer in &song.layers {
            layer.write(writer, version, hooks)?;
        }

        // 自定义乐器部分
        let table = hooks.encode_custom_insts(Cow::Borrowed(song.custom_instruments.as_slice()));
        let count = table.len().min(255) as u8;
        writer.write_u8(count)?;
        for instr in table.iter().take(count as usize) {
            instr.write(writer, (), hooks)?;
        }

        Ok(())
    }
}

impl Codec for Header {
    type Context = ();

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        hooks: &mut M,
    ) -> Result<Self> {
        let mut header = Self::default();

        // 版本
        let song_length = reader.read_u16()?;
        header.version = Version::new(match song_length == 0 {
            true => reader.read_u8()?,
            false => 0,
        })?;

        header.default_instruments = match header.version.get() {
            0 => 10,
            _ => reader.read_u8()?,
        };

        header.song_length = match header.version.get() >= 3 {
            true => reader.read_u16()? as _,
            false => song_length as _,
        };

        // 头部分
        header.song_layers = reader.read_u16()? as _;
        header.song_name = reader.read_string()?;
        header.song_author = reader.read_string()?;
        header.original_author = reader.read_string()?;
        header.description = reader.read_string()?;
        header.tempo = f32::parse(reader, (), hooks)?;
        header.auto_save = reader.read_bool()?;
        header.auto_save_duration = reader.read_u8()? as _;
        header.time_signature = reader.read_u8()?;
        header.minutes_spent = reader.read_u32()?;
        header.left_clicks = reader.read_u32()?;
        header.right_clicks = reader.read_u32()?;
        header.blocks_added = reader.read_u32()?;
        header.blocks_removed = reader.read_u32()?;
        header.song_origin = reader.read_string()?;

        // 循环部分
        if header.version.get() >= 4 {
            header.is_loop = reader.read_bool()?;
            header.max_loop_count = reader.read_u8()? as _;
            header.loop_start = reader.read_u16()? as _;
        }

        Ok(hooks.decode_header(header))
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        hooks: &mut M,
    ) -> Result<()> {
        let header = hooks.encode_header(Cow::Borrowed(self));

        // 版本
        if header.version.get() > 0 {
            writer.write_u16(0)?;
            header.version.write(writer, (), hooks)?;
            writer.write_u8(header.default_instruments)?;
        } else {
            writer.write_u16(header.song_length.max(1).try_into().unwrap_or(u16::MAX))?;
        }

        if header.version.get() >= 3 {
            writer.write_u16(header.song_length.try_into().unwrap_or(u16::MAX))?;
        }

        // 头部分
        writer.write_u16(header.song_layers.try_into().unwrap_or(u16::MAX))?;
        writer.write_string(&header.song_name)?;
        writer.write_string(&header.song_author)?;
        writer.write_string(&header.original_author)?;
        writer.write_string(&header.description)?;
        header.tempo.write(writer, (), hooks)?;
        writer.write_bool(header.auto_save)?;
        writer.write_u8(header.auto_save_duration.try_into().unwrap_or(u8::MAX))?;
        writer.write_u8(header.time_signature)?;
        writer.write_u32(header.minutes_spent)?;
        writer.write_u32(header.left_clicks)?;
        writer.write_u32(header.right_clicks)?;
        writer.write_u32(header.blocks_added)?;
        writer.write_u32(header.blocks_removed)?;
        writer.write_string(&header.song_origin)?;

        // 循环部分
        if header.version.get() >= 4 {
            writer.write_bool(header.is_loop)?;
            writer.write_u8(header.max_loop_count.try_into().unwrap_or(u8::MAX))?;
            writer.write_u16(header.loop_start.try_into().unwrap_or(u16::MAX))?;
        }

        Ok(())
    }
}

// Notes
//
// ++++++++++++============++++++++++++============++++++++++++============

impl Codec for Notes<Position, Note> {
    type Context = (Version, u8);

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<Self> {
        let mut notes = BTreeMap::new();

        // tick
        let mut tick_cursor = Tick::MAX;
        while let Some(tick_jump) = reader.read_jump()? {
            tick_cursor = tick_cursor.wrapping_add(tick_jump.get());

            // layer
            let mut layer_cursor = Index::MAX;
            while let Some(layer_jump) = reader.read_jump()? {
                layer_cursor = layer_cursor.wrapping_add(layer_jump.get());

                let note = Note::parse(reader, context, hooks)?;
                notes.insert(Position::new(tick_cursor, layer_cursor), note);
            }
        }

        Ok(notes.into())
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<()> {
        let mut iter = self.iter().peekable();
        let mut prev_tick = Tick::MAX;
        let mut prev_layer = Index::MAX;

        while let Some((pos, note)) = iter.next() {
            // tick 上升沿
            if pos.into_tick() != prev_tick {
                let tick_jump = pos.into_tick().wrapping_sub(prev_tick);
                writer.write_jump(NonZeroU32::new(tick_jump))?;
            }
            // layer 上升沿
            let layer_jump = pos.into_layer().wrapping_sub(prev_layer);
            writer.write_jump(NonZeroU32::new(layer_jump))?;

            note.write(writer, context, hooks)?;
            prev_tick = pos.into_tick();
            prev_layer = pos.into_layer();
            // layer 下降沿
            let next_tick = iter.peek().map(|(pos, _)| pos.into_tick());
            if next_tick.is_none() || next_tick.unwrap() != pos.into_tick() {
                writer.write_jump(None)?;
                prev_layer = Index::MAX;
            }
        }
        // tick 下降沿
        writer.write_jump(None)?;

        Ok(())
    }
}

// Note
//
// ++++++++++++============++++++++++++============++++++++++++============

impl Codec for Note {
    type Context = (Version, u8);

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<Self> {
        let (version, first_custom_index) = context;
        let mut note = Self::default();
        let instrument = Instrument::parse(reader, first_custom_index, hooks)?;
        let key = Key::parse(reader, (), hooks)?;
        note.tone = Tone::new(instrument, key);

        if version.get() >= 4 {
            note.velocity = Volume::parse(reader, (), hooks)?;
            note.panning = Panning::parse(reader, (), hooks)?;
            note.pitch = reader.read_i16()?;
        }

        let note = hooks.decode_note(note);
        Ok(note)
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        context: Self::Context,
        hooks: &mut M,
    ) -> Result<()> {
        let (version, first_custom_index) = context;
        let note = hooks.encode_note(Cow::Borrowed(self));
        let instrument = note.tone.instrument;
        instrument.write(writer, first_custom_index, hooks)?;
        writer.write_u8(note.tone.key.into())?;

        if version.get() >= 4 {
            note.velocity.write(writer, (), hooks)?;
            note.panning.write(writer, (), hooks)?;
            writer.write_i16(note.pitch)?;
        }

        Ok(())
    }
}

// Layer
//
// ++++++++++++============++++++++++++============++++++++++++============

impl Codec for Layer {
    type Context = Version;

    /// parses a Layer from a reader with version context
    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        version: Self::Context,
        hooks: &mut M,
    ) -> Result<Self> {
        let mut layer = Self::default();
        layer.name = reader.read_string()?;

        if version.get() >= 4 {
            layer.disabled = reader.read_bool()?;
        }

        layer.volume = Volume::parse(reader, (), hooks)?;

        if version.get() >= 2 {
            layer.panning = Panning::parse(reader, (), hooks)?;
        }

        let layer = hooks.decode_layer(layer);
        Ok(layer)
    }

    /// writes a Layer to a writer with version context
    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        version: Self::Context,
        hooks: &mut M,
    ) -> Result<()> {
        let layer = hooks.encode_layer(Cow::Borrowed(self));

        writer.write_string(&layer.name)?;

        if version.get() >= 4 {
            writer.write_bool(layer.disabled)?;
        }

        layer.volume.write(writer, (), hooks)?;

        if version.get() >= 2 {
            layer.panning.write(writer, (), hooks)?;
        }

        Ok(())
    }
}

// Custom Instrument
//
// ++++++++++++============++++++++++++============++++++++++++============

impl Codec for CustomInstrument {
    type Context = ();

    /// parses an Instrument from a reader
    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        hooks: &mut M,
    ) -> Result<Self> {
        let mut instrument = Self::default();
        instrument.name = reader.read_string()?;
        instrument.file = reader.read_string()?;
        instrument.pitch = reader.read_u8()?;
        instrument.press_key = reader.read_bool()?;
        let instrument = hooks.decode_custom_inst(instrument);
        Ok(instrument)
    }

    /// writes an Instrument to a writer
    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        hooks: &mut M,
    ) -> Result<()> {
        let instrument = hooks.encode_custom_inst(Cow::Borrowed(self));
        writer.write_string(&instrument.name)?;
        writer.write_string(&instrument.file)?;
        writer.write_u8(instrument.pitch)?;
        writer.write_bool(instrument.press_key)?;
        Ok(())
    }
}

// Basic Types
//
// ++++++++++++============++++++++++++============++++++++++++============

impl Codec for Version {
    type Context = ();

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        _: &mut M,
    ) -> Result<Self> {
        let version = Version::new(reader.read_u8()?)?;
        Ok(version)
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        _: &mut M,
    ) -> Result<()> {
        Ok(writer.write_u8(self.get())?)
    }
}

impl Codec for Instrument {
    /// Byte index where custom instruments start.
    type Context = u8;

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        first_custom_index: Self::Context,
        hooks: &mut M,
    ) -> Result<Self> {
        debug_assert!(first_custom_index <= Instrument::vanilla_count());
        let byte = reader.read_u8()?;
        let instrument = match byte < first_custom_index {
            true => Instrument::NBS_INDEX[byte as usize],
            false => Instrument::Custom(byte - first_custom_index),
        };
        let instrument = hooks.decode_instrument(instrument);
        Ok(instrument)
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        first_custom_index: Self::Context,
        hooks: &mut M,
    ) -> Result<()> {
        let instrument = hooks.encode_instrument(*self);
        let byte = match instrument {
            Instrument::Custom(slot) => first_custom_index.saturating_add(slot),
            // Instrument::Imitate(_) => unimplemented!(),
            inst => inst
                .vanilla_index()
                .filter(|index| *index < first_custom_index)
                .unwrap_or(0),
        };
        writer.write_u8(byte)?;
        Ok(())
    }
}

impl Codec for Volume {
    type Context = ();

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        _: &mut M,
    ) -> Result<Self> {
        let volume = Volume::new(reader.read_u8()?)?;
        Ok(volume)
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        _: &mut M,
    ) -> Result<()> {
        Ok(writer.write_u8(self.get())?)
    }
}

impl Codec for Key {
    type Context = ();

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        _: &mut M,
    ) -> Result<Self> {
        let key: Key = reader.read_u8()?.into();
        Ok(key)
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        _: &mut M,
    ) -> Result<()> {
        Ok(writer.write_u8((*self).into())?)
    }
}

impl Codec for Panning {
    type Context = ();

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        _: &mut M,
    ) -> Result<Self> {
        let raw = reader.read_u8()?;
        // Convert from file representation (0-200) to internal (-100..100)
        let panning = Panning::new(raw.wrapping_sub(100) as i8)?;
        Ok(panning)
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        _: &mut M,
    ) -> Result<()> {
        // Convert from internal (-100..100) to file representation (0-200)
        Ok(writer.write_u8((self.get() as u8).wrapping_add(100))?)
    }
}

impl Codec for f32 {
    type Context = ();

    fn parse<R: io::Read, M: Transformer + ?Sized>(
        reader: &mut R,
        _: Self::Context,
        _: &mut M,
    ) -> Result<Self> {
        // Convert from u16 to f32 and divide by 100.0
        let value = reader.read_u16()? as f32 / 100.0;
        Ok(value)
    }

    fn write<W: io::Write, M: Transformer + ?Sized>(
        &self,
        writer: &mut W,
        _: Self::Context,
        _: &mut M,
    ) -> Result<()> {
        // Convert f32 to u16 by multiplying by 100.0
        Ok(writer.write_u16((*self * 100.0) as u16)?)
    }
}
