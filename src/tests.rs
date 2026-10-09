use crate::note::{Note, NoteStreamExt, Notes, Tone};
use crate::song::Song;
use crate::types::{LayerAnchor, Position, Tick, TimeAnchor, Version};
use std::collections::BTreeMap;

#[test]
fn test_v6_to_v5_preset_instruments() {
    use crate::note::Instrument;

    let mut song = Song::open_nbs("fixtures/source.nbs").unwrap();

    // 注入一个 v6 独有的原生乐器（Trumpet，索引 16），v5 无法表示
    song.notes.values_mut().next().unwrap().tone.instrument = Instrument::Trumpet;

    song.header.version = Version::new(5).unwrap();
    let path = std::env::temp_dir().join("rsnbs_exp_out_v5.nbs");
    song.save_nbs(&path).unwrap();

    // 写入是纯投影：原 song 不被修改
    assert!(
        song.notes
            .values()
            .any(|n| n.tone.instrument == Instrument::Trumpet)
    );

    // 回读：v5 文件、16 个原生乐器、音符数不变
    let back = Song::open_nbs(&path).unwrap();
    assert_eq!(back.header.version, Version::new(5).unwrap());
    assert_eq!(back.header.default_instruments, 16);
    assert_eq!(back.notes.len(), song.notes.len());

    // 折叠：预设条目还原为原生乐器，不入内存表
    let (_, first_note) = back.notes.iter().next().unwrap();
    assert_eq!(first_note.tone.instrument, Instrument::Trumpet);
    assert!(back.custom_instruments.is_empty());

    // roundtrip 字节稳定：再写一次应与首次写入一致
    let mut out = Vec::new();
    back.write(&mut out).unwrap();
    assert_eq!(out, std::fs::read(&path).unwrap());
}

#[test]
fn test_scale_ticks() {
    let mut song = Song::open_nbs("fixtures/source.nbs").unwrap();

    const NUM: Tick = 3;
    const DEN: Tick = 1;

    // Scale each note's tick by numerator/denominator, multiply first
    let scaled_notes: Notes = song
        .notes
        .into_iter()
        .map(|(pos, note)| {
            let new_tick = pos.into_tick() * NUM / DEN;
            let new_pos = Position::new(new_tick, pos.into_layer());
            (new_pos, note)
        })
        .collect();

    song.notes = scaled_notes;

    // Also update song length metadata if present
    song.header.song_length = song.header.song_length * NUM / DEN;

    song.save_nbs("fixtures/scaled.nbs").unwrap();
}

#[test]
fn test_v6_to_v5_conversion() {
    let mut song_v6 = Song::open_nbs("fixtures/source.nbs").unwrap();

    song_v6.header.version = Version::new(5).unwrap();
    song_v6.save_nbs("fixtures/out_v5.nbs").unwrap();

    // The downgraded file must be a valid v5 file: 16 vanilla instruments,
    // and every note must still parse (trumpets are converted to custom
    // instruments when present).
    let back = Song::open_nbs("fixtures/out_v5.nbs").unwrap();
    assert_eq!(back.header.version, Version::new(5).unwrap());
    assert_eq!(back.header.default_instruments, 16);
    assert_eq!(back.notes.len(), song_v6.notes.len());
}

// cargo test analyze_tones

#[test]
fn analyze_tones() {
    let mut song = Song::open_nbs("fixtures/source.nbs").unwrap();

    let mut by_tone: BTreeMap<Tone, Vec<(Position, Note)>> = Default::default();
    for (pos, note) in song.notes {
        by_tone.entry(note.tone).or_default().push((pos, note));
    }
    let slices: Vec<Notes> = by_tone.into_values().map(|v| Notes::from_iter(v)).collect();

    song.notes = Notes::from_iter(slices.concat());
    song.header.is_loop = true;
    song.save_nbs("fixtures/analyzed.nbs").unwrap();
}
