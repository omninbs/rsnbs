use super::{Song, Binary};
use std::fs::File;
use std::io::Write;
use std::fs::OpenOptions;

macro_rules! impl_writable_bin {
    ([$($int:ty,)*]) => {
        $(
            impl writable_bin for $int {
                fn write_bin(&self, mut file: File) -> Option<()> {
                    match file.write_all(&self.to_le_bytes()) {Ok(x) => x, Err(err) => panic!("{}, {:?}", err, file)};
                    return Some(())
                }

                fn write_default(mut file: File) -> Option<Self> {
                    let default_val = 0 as $ int;

                    default_val.write_bin(file)?;

                    return Some(default_val)
                }
            }
        )*
    };
}

trait writable_bin {
    fn write_bin(&self, file: File) -> Option<()>;
    fn write_default(file: File) -> Option<Self> where Self: Sized;
}

impl_writable_bin!([u8, i8, i16, i32,]);

impl writable_bin for String {
    fn write_bin(&self, mut file: File) -> Option<()> {
        file.write_all(&(self.len() as i32).to_le_bytes()).ok()?;
        
        file.write_all(self.as_bytes()).ok()?;

        return Some(())
    }

    fn write_default(mut file: File) -> Option<Self> {
        let default_val = String::new();

        default_val.write_bin(file)?;

        return Some(default_val)
    }
}

impl writable_bin for bool {
    fn write_bin(&self, mut file: File) -> Option<()> {
        file.write_all(&(match self {&false => 0u8, &true => 1u8}).to_le_bytes()).ok()?; return Some(())
    }

    fn write_default(mut file: File) -> Option<Self> {
        let default_val = false;

        default_val.write_bin(file)?;

        return Some(default_val)
    }
}

fn write_field<T: writable_bin>(file: File, field: &Option<T>) -> Option<()> {
    if let Some(val) = field {val.write_bin(file)?;}
    else {T::write_default(file)?;}
    
    return Some(())
}

fn write_part(mut file: File, part: Vec<(Binary, u8)>) -> Option<()> {
    for (binary, version) in part {
        match binary {
            Binary::Bool(val) => write_field(file.try_clone().ok()?, val)?,
            Binary::Byte(val) => write_field(file.try_clone().ok()?, val)?,
            Binary::UByte(val) => write_field(file.try_clone().ok()?, val)?,
            Binary::Short(val) => write_field(file.try_clone().ok()?, val)?,
            Binary::Integer(val) => write_field(file.try_clone().ok()?, val)?,
            Binary::String(val) => write_field(file.try_clone().ok()?, val)?,
        };
    }

    return Some(())
}

impl Song {
    pub fn save(&mut self, filename: &str, version: u8) -> Option<()> {
        self.header.song_length = Some(self.notes[self.notes.len()-1].tick? as i16);
        self.header.song_layers = Some(self.layers.len() as i16);
        self.header.version = Some(version as i8);

        let mut file = match OpenOptions::new().write(true).open(filename) {
            Ok(file) => file, 
            Err(_err) => {File::create(filename).ok()?; File::open(filename).ok()?}
        };

        write_part(file.try_clone().ok()?, self.header.as_ref_vec(version));
        
        let mut prev_tick = -1;
        let mut prev_layer = -1;
        
        for note in &self.notes {
            if note.tick? - prev_tick > 0 {
                if prev_tick > -1 {write_field(file.try_clone().ok()?, &Some(0i16));}
                write_field(file.try_clone().ok()?, &Some((note.tick? - prev_tick) as i16));
                prev_layer = -1;
            }
            
            write_field(file.try_clone().ok()?, &Some((note.layer? - prev_layer) as i16));
            
            write_field(file.try_clone().ok()?, &note.instrument);
            write_field(file.try_clone().ok()?, &note.key);
            
            if version >= 4 {
                write_field(file.try_clone().ok()?, &note.velocity);
                write_field(file.try_clone().ok()?, &note.panning);
                write_field(file.try_clone().ok()?, &note.pitch);
            }

            prev_tick = note.tick?;
            prev_layer = note.layer?;
        }

        write_field(file.try_clone().ok()?, &Some(0i16));
        write_field(file.try_clone().ok()?, &Some(0i16));

        for layer in &self.layers {
            write_part(file.try_clone().ok()?, layer.as_ref_vec(version));
        }
        
        write_field(file.try_clone().ok()?, &Some(self.instruments.len() as u8));

        for instrument in &self.instruments {
            write_part(file.try_clone().ok()?, instrument.as_ref_vec(version));
        }

        return Some(())
    }
}
