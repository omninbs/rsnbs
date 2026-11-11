//! Provides extension traits for reading and writing NBS format data.

use encoding_rs::WINDOWS_1252;
use std::{io, num::NonZeroU32};

/// provides methods for reading nbs format data (little-endian).
pub(super) trait NbsReadExt: io::Read {
    /// reads a bool (stored as u8, 1 = true, 0 = false).
    fn read_bool(&mut self) -> io::Result<bool> {
        let value = self.read_u8()?;
        Ok(value == 1)
    }

    /// reads a u8.
    fn read_u8(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// reads a u16 (little-endian).
    fn read_u16(&mut self) -> io::Result<u16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// reads an i16 (little-endian).
    fn read_i16(&mut self) -> io::Result<i16> {
        let mut buf = [0; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    /// reads a u32 (little-endian).
    fn read_u32(&mut self) -> io::Result<u32> {
        let mut buf = [0; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// reads a length-prefixed string.
    fn read_string(&mut self) -> io::Result<String> {
        let len = self.read_u32()?;
        let mut buf = vec![0; len as usize];
        self.read_exact(&mut buf)?;
        // 可恶的欧洲人！
        // Decode Windows-1252 encoded string
        let (decoded, _, _) = WINDOWS_1252.decode(&buf);
        Ok(decoded.into())
    }

    /// reads a jump time (stored as u16, 0 = no jump).
    fn read_jump(&mut self) -> io::Result<Option<NonZeroU32>> {
        self.read_u16().map(|time| NonZeroU32::new(time as u32))
    }
}

impl<R: io::Read + ?Sized> NbsReadExt for R {}

/// provides methods for writing nbs format data (little-endian).
pub(super) trait NbsWriteExt: io::Write {
    /// writes a bool (stored as u8, 1 = true, 0 = false).
    fn write_bool(&mut self, value: bool) -> io::Result<()> {
        self.write_u8(if value { 1 } else { 0 })
    }

    /// writes a u8.
    fn write_u8(&mut self, value: u8) -> io::Result<()> {
        self.write_all(&[value])
    }

    /// writes a u16 (little-endian).
    fn write_u16(&mut self, value: u16) -> io::Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    /// writes an i16 (little-endian).
    fn write_i16(&mut self, value: i16) -> io::Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    /// writes a u32 (little-endian).
    fn write_u32(&mut self, value: u32) -> io::Result<()> {
        self.write_all(&value.to_le_bytes())
    }

    /// writes a length-prefixed string.
    fn write_string(&mut self, s: &str) -> io::Result<()> {
        // Damn Europeans...
        // Encode string to Windows-1252
        let (bytes, _, _) = WINDOWS_1252.encode(s);
        let len: u32 = bytes.len().try_into().unwrap_or(u32::MAX);
        self.write_u32(len)?;
        self.write_all(&bytes[..len as usize])
    }

    /// writes a jump time (stored as u16, 0 = no jump).
    fn write_jump(&mut self, time: Option<NonZeroU32>) -> io::Result<()> {
        match time {
            Some(time) => self.write_u16(time.get().try_into().unwrap_or(u16::MAX)),
            None => self.write_u16(0),
        }
    }
}

impl<W: io::Write + ?Sized> NbsWriteExt for W {}
