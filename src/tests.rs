use crate::axiom::event::{EventMap, EventStreamExt, LayerAnchor, Tick, TimeAnchor};
use crate::format::nbs::{Note, Position, Song, Tone, Version};
use std::collections::BTreeMap;

#[test]
fn test_v6_to_v5_preset_instruments() {
    use crate::game::minecraft::Instrument;

    let mut song: Song = Song::open_nbs("fixtures/source.nbs").unwrap();

    // inject a v6-only vanilla instrument (Trumpet, index 16), which v5 cannot express
    song.notes.values_mut().next().unwrap().tone.instrument = Instrument::Trumpet;

    song.header.version = Version::new(5).unwrap();
    let path = std::env::temp_dir().join("rsnbs_exp_out_v5.nbs");
    song.save_nbs(&path).unwrap();

    // writing is a pure projection: the original score is untouched
    assert!(
        song.notes
            .values()
            .any(|n| n.tone.instrument == Instrument::Trumpet)
    );

    // read back: v5 file, 16 vanilla instruments, same note count
    let back: Song = Song::open_nbs(&path).unwrap();
    assert_eq!(back.header.version, Version::new(5).unwrap());
    assert_eq!(back.header.default_instruments, 16);
    assert_eq!(back.notes.len(), song.notes.len());

    // folding: the preset entry becomes vanilla again and leaves the table
    let (_, first_note) = back.notes.iter().next().unwrap();
    assert_eq!(first_note.tone.instrument, Instrument::Trumpet);
    assert!(back.custom_instruments.is_empty());

    // byte-stable roundtrip: writing again matches the first write
    let mut out = Vec::new();
    back.write(&mut out).unwrap();
    assert_eq!(out, std::fs::read(&path).unwrap());
}

#[test]
fn test_scale_ticks() {
    let mut song: Song = Song::open_nbs("fixtures/source.nbs").unwrap();

    const NUM: Tick = 3;
    const DEN: Tick = 1;

    // Scale each note's tick by numerator/denominator, multiply first
    let scaled_notes: EventMap<Position, Note> = song
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
    let mut song_v6: Song = Song::open_nbs("fixtures/source.nbs").unwrap();

    song_v6.header.version = Version::new(5).unwrap();
    song_v6.save_nbs("fixtures/out_v5.nbs").unwrap();

    // The downgraded file must be a valid v5 file: 16 vanilla instruments,
    // and every note must still parse (trumpets are converted to custom
    // instruments when present).
    let back: Song = Song::open_nbs("fixtures/out_v5.nbs").unwrap();
    assert_eq!(back.header.version, Version::new(5).unwrap());
    assert_eq!(back.header.default_instruments, 16);
    assert_eq!(back.notes.len(), song_v6.notes.len());
}

// cargo test analyze_tones

#[test]
fn analyze_tones() {
    let mut song: Song = Song::open_nbs("fixtures/source.nbs").unwrap();

    let mut by_tone: BTreeMap<Tone, Vec<(Position, Note)>> = Default::default();
    for (pos, note) in song.notes {
        by_tone.entry(note.tone).or_default().push((pos, note));
    }
    let slices: Vec<EventMap<Position, Note>> = by_tone
        .into_values()
        .map(|v| EventMap::from_iter(v))
        .collect();

    song.notes = EventMap::from_iter(slices.concat());
    song.header.is_loop = true;
    song.save_nbs("fixtures/analyzed.nbs").unwrap();
}
