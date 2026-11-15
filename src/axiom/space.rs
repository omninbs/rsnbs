//! Where a sound lands in the listener's perception.
//!
//! A space can be a stereo pair, a ring, a room, or a panning column in a file
//! format. All of them resolve onto [`Placement`], which keeps only what a
//! listener hears; the geometry a space adds on top of that stays in the space.

use std::f32::consts::{PI, TAU};

// projection
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A space that resolves onto the listener's perception.
///
/// Everything downstream needs only the placement, so any type that can
/// project itself is a space. `Copy` and `PartialEq` keep one cheap to pass
/// and compare. Projection is total, deterministic, and one way: equal spaces
/// project the same, but spaces projecting the same need not be equal, and a
/// [`Placement`] projects onto itself.
pub trait Spatial: Copy + PartialEq {
    /// projects the space onto the listener's perception.
    fn placement(&self) -> Placement;
}

// placement & azimuth & loudness
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A psychological position: where a sound is heard and how loud.
///
/// Both fields are normalized, so placements that compare equal are heard the
/// same. Elevation, distance, and room are not carried; spaces that have them
/// lose that detail on projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// direction on the horizontal plane.
    pub azimuth: Azimuth,
    /// perceived loudness.
    pub loudness: Loudness,
}

impl Placement {
    /// creates a placement.
    pub const fn new(azimuth: Azimuth, loudness: Loudness) -> Self {
        Self { azimuth, loudness }
    }
}

/// the terminal is already its own projection.
impl Spatial for Placement {
    fn placement(&self) -> Placement {
        *self
    }
}

/// A perceived direction on the listener's horizontal plane, in radians.
///
/// Zero is straight ahead, `-π/2` hard left, `π/2` hard right, and `-π`
/// straight back. The scale follows a standard auditory model, so equal steps
/// are equally noticeable, and a geometric bearing must be mapped first.
/// Values wrap into `[-π, π)`, so `π` folds onto `-π`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Azimuth {
    radians: f32,
}

impl Azimuth {
    /// straight ahead.
    pub const FRONT: Self = Self { radians: 0.0 };
    /// hard left.
    pub const LEFT: Self = Self { radians: -PI / 2.0 };
    /// hard right.
    pub const RIGHT: Self = Self { radians: PI / 2.0 };
    /// straight back, the seam of the circle.
    pub const BACK: Self = Self { radians: -PI };

    /// wraps radians onto the circle; non-finite input becomes [`FRONT`](Self::FRONT).
    pub fn new(radians: f32) -> Self {
        let radians = match radians.is_finite() {
            true => (radians + PI).rem_euclid(TAU) - PI,
            false => 0.0,
        };
        Self { radians }
    }

    /// returns the direction in radians, within `[-π, π)`.
    pub const fn radians(self) -> f32 {
        self.radians
    }
}

/// Perceived loudness relative to full scale.
///
/// The value is a ratio on perception, not on signal amplitude: `0.5` sounds
/// half as loud as `1.0`, so the meter is logarithmic in amplitude. `0` is
/// inaudible and `1` is full scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    ratio: f32,
}

impl Loudness {
    /// inaudible.
    pub const SILENT: Self = Self { ratio: 0.0 };
    /// full scale.
    pub const FULL: Self = Self { ratio: 1.0 };

    /// clamps a perceptual ratio into `[0, 1]`; non-finite input becomes
    /// [`SILENT`](Self::SILENT).
    pub const fn new(ratio: f32) -> Self {
        let ratio = match ratio.is_finite() {
            true => ratio.clamp(0.0, 1.0),
            false => 0.0,
        };
        Self { ratio }
    }

    /// returns the perceptual ratio, within `[0, 1]`.
    pub const fn ratio(self) -> f32 {
        self.ratio
    }

    /// whether the loudness is inaudible.
    pub const fn is_silent(self) -> bool {
        self.ratio == 0.0
    }
}
