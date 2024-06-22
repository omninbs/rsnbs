# rsnbs

[![GitHub Actions](https://github.com/omninbs/rsnbs/workflows/Rust/badge.svg)](https://github.com/omninbs/rsnbs/actions)
[![crates.io](https://img.shields.io/crates/v/rsnbs.svg)](https://crates.io/crates/rsnbs)
[![Rust Version](https://img.shields.io/badge/rust-1.56.0%2B-orange.svg)](https://crates.io/crates/rsnbs)
[![Code style: rustfmt](https://img.shields.io/badge/code%20style-rustfmt-blue.svg)](https://github.com/rust-lang/rustfmt)

> A simple rust library to read and write [.nbs files](https://opennbs.org/nbs)
> from [Open Note Block Studio](https://opennbs.org/).

`rsnbs` has all the functionality `pynbs` has, like iterating over Note Block Studio songs
```rust
use rsnbs::read_nbs;

let song = read_nbs("song.nbs");
for note in song.notes {
  println!("tick: {}, instrument: {}", note.tick.unwrap(), note.instrument.unwrap())
}
```

or generating new songs programmatically
```rust
use rsnbs::{Song, Note};

let song = Song::default();
for i in 0..10 {
  Song.notes.push(Note {
    tick: Some(i), layer: None, instrument: None,
    velocity: 30, panning: None, pitch: None, key: Some(i+35)
  })
}
```
the main difference is that every field is an option (for version differences sake) where None acts as 0/"".

## Installation

The package can be installed with `cargo`
```bash
$ cargo install rsnbs
```

## Reading / Writing
You can use the read_nbs function to read an parse a specific NBS file of any version.
```rust
  let song = rsnbs::read_nbs("song.nbs");
```
This returns a rsnbs::Song object wich can then be written using Song::save
```rust
  song.save("song.nbs", version);
```

## Song struct
Song implements the Default trait so you can instance a default with 1 layer and all None fields
```rust
  let song = Song::default();
```
### Fields
#### Header
the first field is `header`, the file header, of type struct `Header`
Attribute                   | Type    | Details
:---------------------------|:--------|:------------------------------------------------
`header.version`            | `i8`   | The NBS version this file was saved on.
`header.default_instruments`| `i8`   | The amount of instruments from vanilla Minecraft in the song.
`header.song_length`        | `i16`   | The length of the song, measured in ticks.
`header.song_layers`        | `i16`   | The ID of the last layer with at least one note block in it.
`header.song_name`          | `String`   | The name of the song.
`header.song_author`        | `String`   | The author of the song.
`header.original_author`    | `String`   | The original song author of the song.
`header.description`        | `String`   | The description of the song.
`header.tempo`              | `i16` | The tempo of the song multiplied by 100.
`header.auto_save`          | `bool`  | Whether auto-saving has been enabled.
`header.auto_save_duration` | `i8`   | The amount of minutes between each auto-save.
`header.time_signature`     | `i8`   | The time signature of the song.
`header.minutes_spent`      | `i32`   | The amount of minutes spent on the project.
`header.left_clicks`        | `i32`   | The amount of times the user has left-clicked.
`header.right_clicks`       | `i32`   | The amount of times the user has right-clicked.
`header.blocks_added`       | `i32`   | The amount of times the user has added a block.
`header.blocks_removed`     | `i32`   | The amount of times the user has removed a block.
`header.song_origin`        | `String`   | The file name of the original MIDI or schematic.
`header.loop`               | `bool`  | Whether the song should loop back to the start after ending.
`header.max_loop_count`     | `i8`   | The amount of times to loop. 0 = infinite.
`header.loop_start`         | `i16`   | The tick the song will loop back to at the end of playback.

#### Notes
the `notes` field has all the notes of type struct `Note` stored in a vec in order
Attribute         | Type  | Details
:---------------- |:------|:------------------------------------------------
`note.tick`       | `i32` | The tick at which the note plays.
`note.layer`      | `i32` | The ID of the layer in which the note is placed.
`note.instrument` | `i8` | The ID of the instrument.
`note.key`        | `i8` | The key of the note. (between 0 and 87)
`note.velocity`   | `i8` | The velocity of the note. (between 0 and 100)
`note.panning`    | `u8` | The stereo panning of the note. (between -100 and 100)
`note.pitch`      | `i16` | The detune of the note, in cents. (between -1200 and 1200)

#### Layers 
the `layers` field is a vec of all layers of type struct `Layer` in order
Attribute         | Type  | Details
:-----------------|:------|:------------------------
`layer.name`      | `String` | The name of the layer.
`layer.lock`      | `bool`| Whether the layer is locked.
`layer.volume`    | `i8` | The volume of the layer.
`layer.panning`   | `u8` | The stereo panning of the layer.

#### Instruments
the `instruments` field stores all the custom instruments of the song in order in a vec
Attribute              | Type   | Details
:----------------------|:-------|:----------------------------------------------------------
`instrument.name`      | `String`  | The name of the instrument.
`instrument.file`      | `String`  | The name of the sound file of the instrument.
`instrument.pitch`     | `i8`  | The pitch of the instrument. (between 0 and 87)
`instrument.press_key` | `bool` | Whether the piano should automatically press keys with the instrument when the marker passes them.
