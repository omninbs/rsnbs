//! Pitches on equal-tempered grids.

use std::fmt::{self, Display, Formatter};

// pitch
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A scientific representation of a pitch on an equal-tempered grid,
/// independent of its timbre.
///
/// `REFERENCE` fixes the reference pitch and `DIVISIONS` the number of equal
/// steps in an octave; a step counts from the reference. The grid is part of
/// the type, so pitches from different grids never mix.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pitch<const REFERENCE: u32 = 440_000, const DIVISIONS: u8 = 12> {
    /// step counted from the reference pitch.
    pub step: i32,
}

impl<const REFERENCE: u32, const DIVISIONS: u8> Pitch<REFERENCE, DIVISIONS> {
    /// creates a pitch from a step counted from the reference pitch.
    pub const fn new(step: i32) -> Self {
        Self { step }
    }

    /// moves the pitch by `steps` on the same grid.
    pub const fn transpose(self, steps: i32) -> Self {
        Self {
            step: self.step + steps,
        }
    }

    /// returns the frequency in millihertz, rounded to the nearest.
    pub fn frequency_millihz(self) -> u64 {
        let hertz = f64::from(REFERENCE) * f64::exp2(f64::from(self.step) / f64::from(DIVISIONS));
        hertz.round() as u64
    }

    /// re-expresses the pitch on another grid, rounded to the nearest step.
    pub fn retune<const TARGET: u32, const N: u8>(self) -> Pitch<TARGET, N> {
        let octaves = f64::from(self.step) / f64::from(DIVISIONS)
            + (f64::from(REFERENCE) / f64::from(TARGET)).log2();
        Pitch::new((octaves * f64::from(N)).round() as i32)
    }
}

/// Names the pitch on a twelve-step grid, following the absolute frequency.
impl<const REFERENCE: u32> Display for Pitch<REFERENCE, 12> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const NAMES: [&str; 12] = [
            "A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#",
        ];
        let hertz = f64::from(REFERENCE) * f64::exp2(f64::from(self.step) / 12.0);
        let index = (12.0 * (hertz / 440_000.0).log2()).round() as i32;
        let name = NAMES[index.rem_euclid(12) as usize];
        write!(f, "{name}{}", 4 + (index + 9).div_euclid(12))
    }
}

/// Names the pitch on a quarter-tone grid, following the absolute frequency.
impl<const REFERENCE: u32> Display for Pitch<REFERENCE, 24> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        const NAMES: [&str; 24] = [
            "A", "A𝄲", "A#", "A#𝄲", "B", "B𝄲", "C", "C𝄲", "C#", "C#𝄲", "D", "D𝄲", "D#", "D#𝄲", "E",
            "E𝄲", "F", "F𝄲", "F#", "F#𝄲", "G", "G𝄲", "G#", "G#𝄲",
        ];
        let hertz = f64::from(REFERENCE) * f64::exp2(f64::from(self.step) / 24.0);
        let index = (24.0 * (hertz / 440_000.0).log2()).round() as i32;
        let name = NAMES[index.rem_euclid(24) as usize];
        write!(f, "{name}{}", 4 + (index + 18).div_euclid(24))
    }
}

/// Names the pitch on a nineteen-step grid, as steps from the reference.
impl<const REFERENCE: u32> Display for Pitch<REFERENCE, 19> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:+}/19", self.step)
    }
}

/// Names the pitch on a thirty-one-step grid, as steps from the reference.
impl<const REFERENCE: u32> Display for Pitch<REFERENCE, 31> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:+}/31", self.step)
    }
}

/// Names the pitch on a fifty-three-step grid, as steps from the reference.
impl<const REFERENCE: u32> Display for Pitch<REFERENCE, 53> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:+}/53", self.step)
    }
}
