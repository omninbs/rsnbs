mod fields;
use fields::{Header, Note, Layer, Instrument, Binary, BinaryMut};

#[derive(Debug, PartialEq)]
pub struct Song {
    pub header: Header,

    pub notes: Vec<Note>,
    
    pub layers: Vec<Layer>,
    
    pub instruments: Vec<Instrument>
}

pub mod parser;
pub use parser::read_nbs;

mod tests;

mod writer;

impl Default for Song {
    fn default() -> Self {
        return Song {
            header: Header::default(),
            notes: vec!(),
            layers: vec!(Layer {
                name: Some(String::new()), lock: Some(false), 
                volume: Some(1), stereo: Some(0)
            }),
            instruments: vec!()
        }
    }
}
