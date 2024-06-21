mod fields;
use fields::{Header, Note, Layer, Instrument, Binary, BinaryMut};

#[derive(Debug, PartialEq)]
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
