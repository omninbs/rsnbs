//! Codec tests with self-contained byte fixtures (no external .nbs files).
//!
//! The `build` helper writes NBS files byte-by-byte per the OpenNBS spec,
//! so these tests are independent of the repo's gitignored fixtures.

use rsnbs::note::{Instrument, Key, Tone};
use rsnbs::song::Song;
use rsnbs::types::{TimeAnchor, Version};
use std::io::Cursor;

fn write_str(buf: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    buf.extend((b.len() as u32).to_le_bytes());
    buf.extend(b);
}

fn push_note(buf: &mut Vec<u8>, version: u8, layer_jump: u16, inst: u8, key: u8) {
    buf.extend(layer_jump.to_le_bytes());
    buf.push(inst);
    buf.push(key);
    if version >= 4 {
        buf.push(100); // velocity
        buf.push(100); // panning
        buf.extend(0i16.to_le_bytes()); // pitch
    }
}

#[allow(clippy::too_many_arguments)]
fn build(
    version: u8,
    fci: u8,
    song_len: u16,
    layers: u16,
    notes: &[(Option<u16>, u16, u8, u8)], // (tick_jump, layer_jump, instrument byte, key)
    layer_names: &[(&str, u8, u8, u8)],   // (name, lock, volume, panning)
    customs: &[(&str, &str, u8, u8)],     // (name, file, key, press_key)
) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut first_note = true;
    if version > 0 {
        buf.extend(0u16.to_le_bytes()); // new format marker
        buf.push(version);
        buf.push(fci);
    } else {
        buf.extend(song_len.to_le_bytes());
    }
    if version >= 3 {
        buf.extend(song_len.to_le_bytes());
    }
    buf.extend(layers.to_le_bytes());
    write_str(&mut buf, "Test Song");
    write_str(&mut buf, "kemiamu");
    write_str(&mut buf, "");
    write_str(&mut buf, "");
    buf.extend(1000u16.to_le_bytes()); // tempo
    buf.push(0); // autosave
    buf.push(10); // autosave minutes
    buf.push(4); // time signature
    for _ in 0..5 {
        buf.extend(0i32.to_le_bytes());
    }
    write_str(&mut buf, ""); // midi
    if version >= 4 {
        buf.push(0); // loop
        buf.push(0); // max loop count
        buf.extend(0u16.to_le_bytes()); // loop start
    }
    for &(tj, lj, inst, key) in notes {
        if let Some(tj) = tj {
            if !first_note {
                buf.extend(0u16.to_le_bytes()); // end of the previous tick's layers
            }
            buf.extend(tj.to_le_bytes());
        }
        push_note(&mut buf, version, lj, inst, key);
        first_note = false;
    }
    buf.extend(0u16.to_le_bytes()); // end of the last tick's layers
    buf.extend(0u16.to_le_bytes()); // end of notes
    for &(name, lock, vol, pan) in layer_names {
        write_str(&mut buf, name);
        if version >= 4 {
            buf.push(lock);
        }
        buf.push(vol);
        if version >= 2 {
            buf.push(pan);
        }
    }
    buf.push(customs.len() as u8);
    for &(name, file, key, press) in customs {
        write_str(&mut buf, name);
        write_str(&mut buf, file);
        buf.push(key);
        buf.push(press);
    }
    buf
}

fn parse(bytes: Vec<u8>) -> Song {
    let mut cursor = Cursor::new(bytes);
    Song::parse(&mut cursor).unwrap()
}

fn write(song: &mut Song) -> Vec<u8> {
    let mut buf = Vec::new();
    song.write(&mut buf).unwrap();
    buf
}

// A v6 file: trumpets are vanilla (fci = 20), bytes at or above 20 are custom.
#[test]
fn v6_parses_trumpets_as_vanilla_and_bytes_as_custom_slots() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[
            (Some(1), 1, 16, 45), // tick 0, layer 0: byte 16 -> Trumpet (vanilla in v6)
            (None, 2, 21, 45),    // tick 0, layer 2: byte 21 -> custom slot 1
        ],
        &[("Layer", 0, 100, 100)],
        &[("A", "a.ogg", 45, 1), ("B", "b.ogg", 45, 1)],
    );
    let song = parse(bytes);
    assert_eq!(song.header.version.get(), 6);
    assert_eq!(song.header.default_instruments, 20);

    let notes: Vec<_> = song.notes.iter().collect();
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0].1.tone.instrument, Instrument::Trumpet);
    assert_eq!(notes[1].1.tone.instrument, Instrument::Custom(1));
    assert_eq!(song.custom_instruments[1].name, "B");
}

// Regression: a v5 custom instrument at byte 17 must not be misread as a trumpet.
#[test]
fn v5_parses_custom_byte_as_custom_slot_not_trumpet() {
    let bytes = build(
        5,
        16,
        0,
        1,
        &[(Some(1), 1, 17, 45)], // byte 17 >= fci 16 -> custom slot 1
        &[("Layer", 0, 100, 100)],
        &[("A", "a.ogg", 45, 1), ("B", "b.ogg", 45, 1)],
    );
    let song = parse(bytes);
    assert_eq!(song.header.default_instruments, 16);
    let notes: Vec<_> = song.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::Custom(1));
}

// write() borrows the song: derived header fields are computed while
// writing, so the in-memory song is never modified.
#[test]
fn write_does_not_modify_the_song() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[(Some(1), 1, 16, 45)], // Trumpet
        &[("Layer", 0, 100, 100)],
        &[],
    );
    let mut song = parse(bytes.clone());
    let out = write(&mut song);

    // Roundtrip is byte-identical and the song itself is unchanged.
    assert_eq!(out, bytes);
    assert!(song.custom_instruments.is_empty());
    let instruments: Vec<_> = song
        .notes
        .values()
        .map(|note| note.tone.instrument)
        .collect();
    assert_eq!(instruments, [Instrument::Trumpet]);
    assert_eq!(song.header.song_length, 0);
    assert_eq!(song.header.song_layers, 1);
}

// Upgrading a v5 song to v6 must shift custom instrument bytes from 16 + slot
// to 20 + slot.
#[test]
fn v5_to_v6_upgrade_remaps_custom_slots() {
    let bytes = build(
        5,
        16,
        0,
        1,
        &[(Some(1), 1, 17, 45)], // custom slot 1: "B"
        &[("Layer", 0, 100, 100)],
        &[("A", "a.ogg", 45, 1), ("B", "b.ogg", 45, 1)],
    );
    let mut song = parse(bytes);
    song.header.version = Version::new(6).unwrap();

    let out = write(&mut song);
    assert_eq!(out[3], 20, "v6 file must claim 20 vanilla instruments");

    let upgraded = parse(out);
    let notes: Vec<_> = upgraded.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::Custom(1));
    assert_eq!(upgraded.custom_instruments[1].name, "B");
}

#[test]
fn new_song_defaults_to_v6_with_20_vanilla_instruments() {
    let song: Song = Song::new();
    assert_eq!(song.header.version.get(), 6);
    assert_eq!(song.header.default_instruments, 20);
}

#[test]
fn v6_roundtrip_is_byte_identical() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[
            (Some(1), 1, 18, 45), // Trumpet
            (None, 2, 20, 45),    // custom slot 0
        ],
        &[("Layer", 0, 100, 100)],
        &[("Custom", "custom.ogg", 45, 1)],
    );
    let mut song = parse(bytes.clone());
    assert_eq!(write(&mut song), bytes);
}

// v5（fci=16）写入会全量预设 4 个小号；读回折叠后表只剩用户条目，
// 再写与首次写出字节一致（写→读→写幂等）。
#[test]
fn v5_roundtrip_is_idempotent() {
    let bytes = build(
        5,
        16,
        0,
        1,
        &[(Some(1), 1, 16, 45)], // custom slot 0
        &[("Layer", 0, 100, 100)],
        &[("Custom", "custom.ogg", 45, 1)],
    );
    let mut song = parse(bytes);
    let first = write(&mut song);

    // 读回折叠：预设条目不入内存表
    let mut back = parse(first.clone());
    assert_eq!(back.custom_instruments.len(), 1);
    let notes: Vec<_> = back.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::Custom(0));

    // 幂等：再写与首次写出字节一致
    let second = write(&mut back);
    assert_eq!(second, first);
}

// v1（fci=16）写入同样全量预设 4 个小号，写→读→写幂等。
#[test]
fn v1_roundtrip_is_idempotent() {
    let bytes = build(
        1,
        16,
        0,
        1,
        &[(Some(1), 1, 3, 45)], // no velocity/panning/pitch in v1
        &[("Layer", 0, 100, 100)],
        &[("Custom", "custom.ogg", 45, 1)],
    );
    let mut song = parse(bytes.clone());
    assert_eq!(song.header.version.get(), 1);
    let first = write(&mut song);
    let mut back = parse(first.clone());
    assert_eq!(back.custom_instruments.len(), 1);
    let second = write(&mut back);
    assert_eq!(second, first);
}

#[test]
fn classic_v0_parses_and_roundtrips() {
    // The first u16 is the song length in the classic format and must stay
    // consistent with the derived song length (the last note's tick).
    let bytes = build(
        0,
        0,
        4,
        1,
        &[(Some(5), 1, 3, 45)], // tick 4: snare drum, no velocity/panning/pitch
        &[("Layer", 0, 100, 100)],
        &[("Custom", "custom.ogg", 45, 1)],
    );
    let mut song = parse(bytes.clone());
    assert_eq!(song.header.version.get(), 0);
    assert_eq!(song.header.default_instruments, 10);
    let notes: Vec<_> = song.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::SnareDrum);

    // v0（fci=10）写出的表尾追加 10 条预设；读回折叠后表只剩用户条目，
    // 再写与首次写出字节一致
    let first = write(&mut song);
    let mut back = parse(first.clone());
    assert_eq!(back.custom_instruments.len(), 1);
    let second = write(&mut back);
    assert_eq!(second, first);
}

// v6 写入（fci=20）无预设：用户自定义的 "Trumpet" 条目不被折叠，保持 Custom。
// v6 → v5 降级：Trumpet 写入预设区域（表尾追加定义），读回折叠还原为 vanilla。
#[test]
fn v6_to_v5_downgrade_presets_trumpet_and_folds_on_read() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[(Some(1), 1, 16, 45)], // Trumpet
        &[("Layer", 0, 100, 100)],
        &[],
    );
    let mut song = parse(bytes);
    song.header.version = Version::new(5).unwrap();

    let out = write(&mut song);
    assert_eq!(out[3], 16, "v5 file must claim 16 vanilla instruments");

    // 折叠：预设条目还原为原生乐器，不入内存表
    let downgraded = parse(out);
    assert!(downgraded.custom_instruments.is_empty());
    let notes: Vec<_> = downgraded.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::Trumpet);

    // 幂等：再写一次与首次写入字节一致
    let mut again = Vec::new();
    downgraded.write(&mut again).unwrap();
    let mut first = Vec::new();
    song.write(&mut first).unwrap();
    assert_eq!(again, first);
}

// 降级 + 已有自定义乐器：预设槽位偏移到现有表尾之后，读回压缩保序重映射。
#[test]
fn v6_to_v5_downgrade_mixes_preset_and_user_customs() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[
            (Some(1), 1, 16, 45), // tick 0: Trumpet → 预设
            (Some(1), 1, 21, 45), // tick 1: Custom(1): "B"
            (Some(1), 1, 20, 45), // tick 2: Custom(0): "A"
        ],
        &[("Layer", 0, 100, 100)],
        &[("A", "a.ogg", 45, 1), ("B", "b.ogg", 45, 1)],
    );
    let mut song = parse(bytes);
    song.header.version = Version::new(5).unwrap();

    let out = write(&mut song);
    let downgraded = parse(out);
    // 表只剩用户条目（预设条目被折叠移除）
    let names: Vec<&str> = downgraded
        .custom_instruments
        .iter()
        .map(|ci| ci.name.as_str())
        .collect();
    assert_eq!(names, ["A", "B"]);
    // 音符：Trumpet 还原 vanilla，自定义槽位保持
    let notes: Vec<_> = downgraded.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::Trumpet);
    assert_eq!(notes[1].1.tone.instrument, Instrument::Custom(1));
    assert_eq!(notes[2].1.tone.instrument, Instrument::Custom(0));
}

// v6 写入（fci=20）无预设：用户自定义的 "Trumpet" 条目不被折叠，保持 Custom。
#[test]
fn v6_roundtrip_keeps_user_custom_named_like_trumpet() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[(Some(1), 1, 20, 45)], // Custom(0): "Trumpet"
        &[("Layer", 0, 100, 100)],
        &[("Trumpet", "trumpet.ogg", 45, 1)],
    );
    let mut song = parse(bytes.clone());
    assert_eq!(write(&mut song), bytes);
    let notes: Vec<_> = song.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::Custom(0));
}

// v0 降级（fci=10）：铁琴等 10 个乐器全量预设，读回折叠还原。
#[test]
fn v0_downgrade_presets_all_instruments_above_fci() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[(Some(5), 1, 10, 45)], // tick 4: Iron Xylophone (index 10)
        &[("Layer", 0, 100, 100)],
        &[],
    );
    let mut song = parse(bytes);
    song.header.version = Version::new(0).unwrap();

    let out = write(&mut song);
    let downgraded = parse(out);
    assert_eq!(downgraded.header.default_instruments, 10);
    assert!(downgraded.custom_instruments.is_empty());
    let notes: Vec<_> = downgraded.notes.iter().collect();
    assert_eq!(notes[0].1.tone.instrument, Instrument::IronXylophone);
}

// 折叠保持音符位置：多 tick 多 layer 的音符在折叠后位置不变。
#[test]
fn fold_preserves_note_positions() {
    let bytes = build(
        6,
        20,
        0,
        2,
        &[
            (Some(1), 1, 16, 45), // tick 0, layer 0: Trumpet → 预设
            (Some(3), 1, 21, 45), // tick 3, layer 0: Custom(1)
            (Some(1), 2, 20, 45), // tick 4, layer 1: Custom(0)
        ],
        &[("L1", 0, 100, 100), ("L2", 0, 100, 100)],
        &[("A", "a.ogg", 45, 1), ("B", "b.ogg", 45, 1)],
    );
    let mut song = parse(bytes);
    song.header.version = Version::new(5).unwrap();

    let out = write(&mut song);
    let downgraded = parse(out);
    let ticks: Vec<_> = downgraded
        .notes
        .iter()
        .map(|(p, _)| p.into_tick())
        .collect();
    assert_eq!(ticks, [0, 3, 4]);
}

#[test]
fn song_of_tones_reads_and_writes() {
    let bytes = build(
        6,
        20,
        0,
        1,
        &[(Some(1), 1, 0, 45)],
        &[("Layer", 0, 100, 100)],
        &[],
    );
    let mut cursor = Cursor::new(bytes.clone());
    let song: Song<Tone> = Song::parse(&mut cursor).unwrap();

    let (_, tone) = song.notes.iter().next().unwrap();
    assert_eq!(*tone, Tone::new(Instrument::Harp, Key::FS4));

    // projecting back to notes restores the defaults the tones dropped
    let mut out = Vec::new();
    song.write(&mut out).unwrap();
    assert_eq!(out, bytes);
}
