use super::{Song, BinaryMut, Layer, Header, Note, Instrument};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

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
    
    let mut string: Vec<u8> = vec!();

    for _ in 0..len {string.push(read_bytes(file.try_clone().ok()?, 1)?[0]);}

    return Some(String::from_utf8(string).ok()?)
}

fn read_binary(file: File, binary: BinaryMut) -> Option<()> {
    match binary {
        BinaryMut::Bool(bool) => {*bool = Some(match read_bytes(file.try_clone().ok()?, 1)?[0] {1 => true, 0 => false, _ => false});},            
        BinaryMut::Byte(byte) => {*byte = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
        BinaryMut::UByte(ubyte) => {*ubyte = Some(read_bytes(file.try_clone().ok()?, 1)?[0]);},
        BinaryMut::Short(short) => {*short = Some(i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?));},
        BinaryMut::Integer(integer) => {*integer = Some(i32::from_bytes(read_bytes(file.try_clone().ok()?, 4)?));},
        BinaryMut::String(string) => {*string = read_string(file.try_clone().ok()?);}
    } return Some(())
}

fn read_nbs_part(file: File, part: Vec<(BinaryMut, u8)>) -> Option<()> {
    for (binary, version) in part {
        read_binary(file.try_clone().ok()?, binary);
    }
    return Some(())
}

pub fn read_nbs(filepath: &str) -> Option<Song> {
    let mut file = File::open(filepath).ok()?;
    
    let mut song = Song::default();
    
    if i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?) <= 0i16 {
        song.header.version = Some(read_bytes(file.try_clone().ok()?, 2)?[0] as i8);
    } else {song.header.version = Some(0i8)}
    

    let version = song.header.version.clone()? as u8;
    file.seek(SeekFrom::Start(0)).ok()?;

    read_nbs_part(file.try_clone().ok()?, song.header.as_mut_vec(version)); 

    song.header.version = Some(version as i8);
    
    if song.header.classic_length? > 0 {song.header.song_length = song.header.classic_length.clone()}

    let mut note = Note::default();
    let mut i = 0i8;
    let mut layer = -1i32;
    let mut tick = -1i32;

    while true {
        match i {
            0 => {
                let tick_change = i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?);
                tick += tick_change as i32; note.tick = Some(tick);
                if tick_change == 0 {break}
            },
            1 => {
                let layer_change = i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?);
                layer += layer_change as i32; note.layer = Some(layer); 
                if layer_change == 0 {i = -1; layer = -1;} else {note.tick = Some(tick);}
            },
            2 => {note.instrument = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
            3 => {
                note.key = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);
                if version < 4 {i = 6;}
            },
            4 => {note.velocity = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
            5 => {note.panning = Some(read_bytes(file.try_clone().ok()?, 1)?[0]);},
            6 => {note.pitch = Some(i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?));},
            _ => {panic!(":skull:")}
        }        
       
        if i == 6 {
            i = 1;
            song.notes.push(note);
            note = Note::default();
        }
        else {i += 1;}
    }
    
    song.layers = vec!();
    let layers = song.header.song_layers.clone();
    for _ in 0..layers? {
        let mut layer = Layer::default();
        read_nbs_part(file.try_clone().ok()?, layer.as_mut_vec(version));
        song.layers.push(layer); 
    }
    
    let instruments = read_bytes(file.try_clone().ok()?, 1)?[0];

    for _ in 0..instruments {
        let mut instrument = Instrument::default();
        read_nbs_part(file.try_clone().ok()?, instrument.as_mut_vec(version));
        song.instruments.push(instrument);
    }

    return Some(song)
}
