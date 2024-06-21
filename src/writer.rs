use super::{Song, Binary};
use std::fs::File;
use std::io::Write;

macro_rules! impl_writable_bin {
    ([$($int:ty,)*]) => {
        $(
            impl writable_bin for $int {
                fn write_bin(&self, mut file: File) -> Option<()> {
                    file.write_all(&self.to_le_bytes()).ok()?;
                    return Some(())
                }

                fn write_default(mut file: File) -> Self {
                    let default_val = 0 as $ int;

                    default_val.write_bin(file);

                    return default_val
                }
            }
        )*
    };
}

trait writable_bin {
    fn write_bin(&self, file: File) -> Option<()>;
    fn write_default(file: File) -> Self;
}

impl_writable_bin!([u8, i8, i16, i32,]);

impl writable_bin for String {
    fn write_bin(&self, mut file: File) -> Option<()> {
        file.write_all(&(self.len() as i32).to_le_bytes()).ok()?;
        
        file.write_all(self.as_bytes()).ok()?;

        return Some(())
    }

    fn write_default(mut file: File) -> Self {
        let default_val = String::new();

        default_val.write_bin(file);

        return default_val
    }
}

fn write_field<T: writable_bin>(field: Option<T>) -> Option<()> {
    
    
    return Some(())
}

impl Song {
    fn save(&self, file: &str, version: u8) -> Option<()> {todo!()}
}
