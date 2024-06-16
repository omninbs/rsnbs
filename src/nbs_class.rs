pub struct NoteBlockSong {
    header: NoteBlockSongHeader,

    notes: Vec<NoteBlockSongNote>,
    
    layers: Vec<NoteBlockSongLayer>,
    
    instrument_count: i16,
    instruments: Vec<NoteBlockSongInstrument>
}

struct NoteBlockSongHeader {
    version: Option<i8>,
    default_instruments: Option<i8>,
    song_length: Option<i16>,
    song_layers: i16,
    song_name: String,
    song_author: String,
    original_author: String,
    description: String,
    tempo: i16,
    auto_save: bool,
    auto_saving_duration: i8,
    time_signature: i8,
    minutes_spent: i32,
    left_clicks: i32,
    right_clicks: i32,
    blocks_added: i32,
    blocks_removed: i32,
    og_file: String,
    r#loop: Option<bool>,
    max_loop_count: Option<i8>,
    loop_start: Option<i16>
}

struct NoteBlockSongNote {
    tick: i16,
    layer: i16,
    instrument: i8,
    key: i8,
    velocity: Option<i8>,
    panning: Option<u8>,
    pitch: Option<i16>
}

struct NoteBlockSongLayer {
    id: i16,
    name: String,
    lock: Option<bool>,
    volume: Option<i8>,
    panning: Option<u8>
}

struct NoteBlockSongInstrument {
    id: i16,
    name: String,
    file: String,
    pitch: i8,
    press_key: bool
}
