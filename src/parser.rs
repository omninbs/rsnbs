use super::{Song, Binary};
use std::fs::File;
use std::io::Read;

//fn read_binary<T>(file: File, r#type: Binary) -> u8 { return 0;
  //  match r#type {
    //    Binary::Bool => {let mut buffer: [u8;1]; file.read_exact(&mut buffer).ok()?; return Some(match buffer[1] {1 => true, 0 => false, _ => return None})}
      //  Binary::Byte => {let mut buffer: [u8;1]; file.read_exact(&mut buffer).ok()?; return Some(buffer[1] as i8)}
        //Binary::UByte => {let mut buffer: [u8;1]; file.read_exact(&mut buffer).ok()?; return Some(buffer[1])}
//        Binary::Short => {let mut buffer: [u8;2]; file.read_exact(&mut buffer).ok()?; return Some(i16::from_le_bytes(buffer))}
  //      Binary::Integer => {let mut buffer: [u8;4]; file.read_exact(&mut buffer).ok()?; return Some(i32::from_le_bytes(buffer))}
      //  Binary::String => {
    //        let string: Vec<u8> = vec!();
            
        //    let len = read_binary(file, Binary::Byte)?;

          //  for _ in 0 .. len {
            //    string.push(read_binary(file, Binary::UByte)?)
   //         }

     //       return Some(String::from_utf8(string).ok()?)
       // }
//    }
    
//}

fn read_bytes(mut file: File, byte_count: u8) -> Option<Vec<u8>> {
    let mut bytes: Vec<u8> = vec!();
    
    let mut byte: [u8; 1] = [0];

    for _ in 0..byte_count {file.read_exact(&mut byte).ok()?; bytes.push(byte[0]);}

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

fn read_nbs_part(file: File, part: Vec<(Binary, u8)>) -> Option<()> {
    for (binary, version) in part {
        match binary {
            Binary::Bool(bool) => {*bool = Some(match read_bytes(file.try_clone().ok()?, 1)?[0] {1 => true, 0 => false, _ => false});},
            Binary::Byte(byte) => {*byte = Some(read_bytes(file.try_clone().ok()?, 1)?[0] as i8);},
            Binary::UByte(ubyte) => {*ubyte = Some(read_bytes(file.try_clone().ok()?, 1)?[0]);},
            Binary::Short(short) => {*short = Some(i16::from_bytes(read_bytes(file.try_clone().ok()?, 2)?));},
            Binary::Integer(integer) => {*integer = Some(i32::from_bytes(read_bytes(file.try_clone().ok()?, 4)?));},
            Binary::String(string) => {*string = read_string(file.try_clone().ok()?);}
        }
    }
    return Some(())
}

pub fn read_nbs(filepath: &str) -> Result<Song, std::io::Error> {
    let file = File::open(filepath)?;

    let mut song = Song::default();

    read_nbs_part(file.try_clone()?, song.header.as_mut_vec());
    
    return Ok(song)
}
