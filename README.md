# rsnbs

![Crates.io Version](https://img.shields.io/crates/v/rsnbs?style=flat-square&logo=rust&labelColor=363a4f&color=f5a97f)
![Crates.io Dependents](https://img.shields.io/crates/dependents/rsnbs?style=flat-square&logo=rust&labelColor=363a4f&color=ee99a0)
![GitHub last commit](https://img.shields.io/github/last-commit/omninbs/rsnbs?style=flat-square&logo=github&labelColor=363a4f&color=f5bde6)
![GitHub Repo stars](https://img.shields.io/github/stars/omninbs/rsnbs?style=flat-square&logo=github&labelColor=363a4f&color=c6a0f6)

> A simple Rust library for working with [.nbs files](https://opennbs.org/nbs) from [Open Note Block Studio](https://opennbs.org/).

This library references [pynbs](https://github.com/OpenNBS/pynbs) and implements its basic functionality, aiming to serve as a fundamental NBS file processing library. Currently supports version 6 of the NBS standard.

However, due to language differences, some adaptations have been made, so behavior may not always be consistent. Since this project is quite niche, it hasn't been thoroughly tested. If you encounter any issues or have feature requests, please submit an issue.

_This crate is in early development with a frequently changing API. Please pin your dependency to a specific version._

```rust
use rsnbs::note::{Instrument, Key, Note, Tone};
use rsnbs::song::Song;
use rsnbs::types::{Position, Tick, TimeAnchor};

// example 1: iterating over a song's notes
let song: Song = Song::open_nbs("fixtures/source.nbs").unwrap();
for (pos, note) in &song.notes {
    let tick: Tick = pos.into_tick();
    let key: Key = note.tone.key;
    println!("tick: {tick}, key: {key}");
}

// example 2: generating a new song programmatically
let mut song: Song = Song::new();
song.header.is_loop = true;
for i in 0..25 {
    let pos: Position = Position::new(i, 0);
    let key: Key = Key::from_minecraft_note(i).unwrap();
    let tone: Tone = Tone::new(Instrument::Harp, key);
    song.notes.insert(pos, Note::new(tone));
}
song.save_nbs("fixtures/generated_example.nbs").unwrap();
```
