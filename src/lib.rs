mod fields;
use fields::{NoteBlockSongHeader, NoteBlockSongNote, NoteBlockSongLayer, NoteBlockSongInstrument};

pub struct NoteBlockSong {
    header: NoteBlockSongHeader,

    notes: Vec<NoteBlockSongNote>,
    
    layers: Vec<NoteBlockSongLayer>,
    
    instrument_count: i16,
    instruments: Vec<NoteBlockSongInstrument>
}

mod read;
use read::read_nbs;

mod write;

pub fn read(file: &str) -> Result<NoteBlockSong, std::io::Error> {todo!()}

pub fn new(name: &str) -> NoteBlockSong {todo!()}
