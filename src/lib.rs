mod fields;
use fields::{Header, Note, Layer, Instrument, Binary};

pub struct Song {
    header: Header,

    notes: Vec<Note>,
    
    layers: Vec<Layer>,
    
    instruments: Vec<Instrument>
}

pub mod parser;
pub use parser::read_nbs;

mod tests;

mod writer;

pub fn read(file: &str) -> Result<Song, std::io::Error> {return read_nbs(file)}

impl Default for Song {
    fn default() -> Self {
        return Song {
            header: Header::default(),
            notes: vec!(),
            layers: vec!(Layer {
                id: Some(0), name: Some(String::new()),
                lock: Some(false), volume: Some(1), stereo: Some(0)
            }),
            instruments: vec!()
        }
    }
}
