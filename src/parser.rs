use super::{Song, Binary, Layer, Header, Note, Instrument};
use std::fs::File;
use std::io::Read;

fn read_bytes(mut file: File, byte_count: u8) -> Option<Vec<u8>> {
    let mut bytes: Vec<u8> = vec!();
    
    let mut byte: [u8; 1] = [0];

    for _ in 0..byte_count {match file.read_exact(&mut byte) {Ok(x) => x, Err(err) => panic!("{}", err)}; bytes.push(byte[0]);}

    return Some(bytes)
}

use std::mem::transmute;

trait from_bytes<T> {fn from_bytes(bytes: Vec<u8>) -> T;}
impl from_bytes<i16> for i16 {fn from_bytes(bytes: Vec<u8>) -> i16 {return unsafe { std::ptr::read(bytes.as_ptr() as *const _) };}}
impl from_bytes<i32> for i32 {fn from_bytes(bytes: Vec<u8>) -> i32 {return unsafe { std::ptr::read(bytes.as_ptr() as *const _) };}}

fn read_string(mut file: File) -> Option<String> {
    let len = i32::from_bytes(read_bytes(file.try_clone().ok()?, 4)?);
    
    println!("{}", len);

    let mut string: Vec<u8> = vec!();

    for _ in 0..len {string.push(read_bytes(file.try_clone().ok()?, 1)?[0]);}

    return Some(String::from_utf8(string).ok()?)
}

fn read_binary(file: File, binary: Binary) -> Option<()> {
    match binary {
        Binary::Bool(bool) => {*bool = Some(match read_bytes(file.try_clone().ok()?, 1)?[0] {1 => true, 0 => false, _ => false});},            Binary::Byte(byte) => {*byte = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
        Binary::UByte(ubyte) => {*ubyte = Some(read_bytes(file.try_clone().ok()?, 1)?[0]);},
        Binary::Short(short) => {*short = Some(i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?));},
        Binary::Integer(integer) => {*integer = Some(i32::from_bytes(read_bytes(file.try_clone().ok()?, 4)?));},
        Binary::String(string) => {*string = read_string(file.try_clone().ok()?);}
    } return Some(())
}

fn read_nbs_part(file: File, part: Vec<(Binary, u8)>) -> Option<()> {
    for (binary, version) in part {
        read_binary(file.try_clone().ok()?, binary);
    }
    return Some(())
}

pub fn read_nbs(filepath: &str) -> Option<Song> {
    let file = File::open(filepath).ok()?;

    let mut song = Song::default();

    read_nbs_part(file.try_clone().ok()?, song.header.as_mut_vec()); 
    
    println!("{}", song.header.version?);

    let mut note = Note::default();
    let mut i = 0i8;
    let mut layer = -1;
    let mut tick = -1;

    while true {
        match i {
            0 => {
                note.next_tick = Some(i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?));
                tick += note.next_tick?; note.tick = Some(tick);
                if note.next_tick == Some(0) {break}
            },
            1 => {
                note.next_layer = Some(i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?));
                layer += note.next_layer?; note.layer = Some(layer); 
                if note.next_layer == Some(0) {i = -1; layer = -1;}
            },
            2 => {note.instrument = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
            3 => {note.key = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
            4 => {note.velocity = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
            5 => {note.panning = Some(read_bytes(file.try_clone().ok()?, 1)?[0]);},
            6 => {note.pitch = Some(i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?));},
            _ => {panic!(":skull:")}
        }        
       
        if i == 6 {
            i = 1;
            println!("{:?}", note);
            song.notes.push(note);
            note = Note::default();
        }
        else {i += 1;}
    }

    song.layers = vec!();
    let layers = song.header.song_layers.clone();
    for _ in 0..layers? {
        let mut layer = Layer::default();
        read_nbs_part(file.try_clone().ok()?, layer.as_mut_vec());
        song.layers.push(layer); 
    }
    
    let instruments = read_bytes(file.try_clone().ok()?, 1)?[0];

    for _ in 0..instruments {
        let mut instrument = Instrument::default();
        read_nbs_part(file.try_clone().ok()?, instrument.as_mut_vec());
        song.instruments.push(instrument);
    }

    return Some(song)
}
