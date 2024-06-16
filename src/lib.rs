mod nbs_class;
use nbs_class::NoteBlockSong;

pub fn read(file: &str) -> Result<NoteBlockSong, io::Error> {todo!()}

pub fn save(nbs: NoteBlockSong, version: u8) -> Result<(), io::Error> {todo!()}

pub fn new(name: &str) -> NoteBlockSong {todo!()}
