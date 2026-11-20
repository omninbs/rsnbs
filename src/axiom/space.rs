//! Where a sound lands in the listener's perception.
//!
//! A space states itself in two standard layouts: [`Placement`] gives a
//! position directly, and [`Stereo`] gives one loudness per channel. The
//! position is built from [`Azimuth`] and [`Loudness`], each of which carries a
//! second standard measure of its own.

use std::f32::consts::{PI, TAU};

// spatial
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A space that resolves onto both standard layouts.
///
/// Projection is lossy and one way: a space may carry elevation, distance, or
/// room that neither terminal records. The channel split keeps only the
/// left-right component, so front and back resolve alike. A terminal projects
/// onto itself.
pub trait Spatial: Copy + PartialEq {
    /// returns the position.
    fn placement(&self) -> Placement;
    /// returns the channel split, derived from [`placement`](Self::placement).
    fn stereo(&self) -> Stereo {
        self.placement().stereo()
    }
}

// loud
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A type that resolves onto both standard measures of loudness.
///
/// A source states one loudness twice: as a linear gain and as a perceived
/// ratio. The two answers must name the same level, so an implementation has
/// to keep them consistent.
pub trait Loud: Copy + PartialEq {
    /// returns the linear gain.
    fn amplitude(&self) -> Amplitude;
    /// returns the perceived ratio, derived from [`amplitude`](Self::amplitude).
    fn loudness(&self) -> Loudness {
        self.amplitude().loudness()
    }
}

// panned
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A type that resolves onto both standard measures of pan.
///
/// A source states one direction twice: as an angle and as a left-right axis.
/// The axis folds front and back together, so the two answers agree only up to
/// that fold.
pub trait Panned: Copy + PartialEq {
    /// returns the direction as an angle.
    fn azimuth(&self) -> Azimuth;
    /// returns the direction as a left-right axis, derived from
    /// [`azimuth`](Self::azimuth).
    fn axis(&self) -> Axis {
        self.azimuth().axis()
    }
}

// placement
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A psychological position: a direction and a loudness.
///
/// Both fields are in the standard scales, so placements that compare equal are
/// heard the same. Elevation, distance, and room are not carried.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement<P = Azimuth, L = Loudness> {
    /// direction in the standard pan scale.
    pub pan: P,
    /// loudness in the standard loudness scale.
    pub loudness: L,
}

impl<P, L> Placement<P, L> {
    /// creates a placement.
    pub const fn new(pan: P, loudness: L) -> Self {
        Self { pan, loudness }
    }
}

/// a placement resolves onto the standard position.
impl<P: Panned, L: Loud> Spatial for Placement<P, L> {
    fn placement(&self) -> Placement {
        Placement::new(self.pan.azimuth(), self.loudness.loudness())
    }

    fn stereo(&self) -> Stereo {
        let amplitude = self.loudness.amplitude().get();
        let axis = self.pan.axis().get();
        let left = amplitude * ((1.0 - axis) / 2.0).sqrt();
        let right = amplitude * ((1.0 + axis) / 2.0).sqrt();
        Stereo::new(
            Amplitude::new(left).loudness(),
            Amplitude::new(right).loudness(),
        )
    }
}

// stereo
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A stereo field: one loudness per channel.
///
/// The channels carry no direction on their own, so the split resolves onto a
/// placement by comparing the two sides.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stereo<L = Loudness> {
    /// loudness of the left channel.
    pub left: L,
    /// loudness of the right channel.
    pub right: L,
}

impl<L> Stereo<L> {
    /// creates a stereo field.
    pub const fn new(left: L, right: L) -> Self {
        Self { left, right }
    }
}

/// a stereo field resolves onto the placement its channels describe.
///
/// The channels are treated as incoherent, so their energies add.
impl<L: Loud> Spatial for Stereo<L> {
    fn placement(&self) -> Placement {
        let left = self.left.amplitude().get();
        let right = self.right.amplitude().get();
        let energy = left * left + right * right;
        let axis = match energy {
            0.0 => 0.0,
            total => (right * right - left * left) / total,
        };
        Placement::new(
            Azimuth::new(axis.asin()),
            Amplitude::new(energy.sqrt()).loudness(),
        )
    }

    fn stereo(&self) -> Stereo {
        Stereo::new(self.left.loudness(), self.right.loudness())
    }
}

// amplitude
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A linear amplitude gain.
///
/// `0` is silence and `1` is unity gain. Values above `1` are legal gains that
/// the output has to limit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Amplitude(f32);

impl Amplitude {
    /// silence.
    pub const SILENT: Self = Self(0.0);
    /// unity gain: the output matches the input.
    pub const UNITY: Self = Self(1.0);

    /// forces a gain to be non-negative; non-finite input becomes [`SILENT`](Self::SILENT).
    pub fn new(amplitude: f32) -> Self {
        let amplitude = match amplitude.is_finite() {
            true => amplitude.max(0.0),
            false => 0.0,
        };
        Self(amplitude)
    }

    /// returns the gain.
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// the gain states itself both ways.
impl Loud for Amplitude {
    fn amplitude(&self) -> Amplitude {
        *self
    }

    fn loudness(&self) -> Loudness {
        Loudness::new(self.0.powf(GAMMA))
    }
}

// loudness
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A perceived loudness ratio.
///
/// The ratio is on perception: `0.5` sounds half as loud as `1.0`, so the
/// scale is logarithmic in amplitude. `0` is inaudible and `1` is unity gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness(f32);

impl Loudness {
    /// inaudible.
    pub const SILENT: Self = Self(0.0);
    /// unity gain: the output matches the input.
    pub const UNITY: Self = Self(1.0);

    /// forces a ratio to be non-negative; non-finite input becomes [`SILENT`](Self::SILENT).
    pub fn new(ratio: f32) -> Self {
        let ratio = match ratio.is_finite() {
            true => ratio.max(0.0),
            false => 0.0,
        };
        Self(ratio)
    }

    /// returns the perceived ratio.
    pub const fn get(self) -> f32 {
        self.0
    }

    /// whether the loudness is inaudible.
    pub const fn is_silent(self) -> bool {
        self.0 == 0.0
    }
}

/// the ratio states itself both ways.
impl Loud for Loudness {
    fn amplitude(&self) -> Amplitude {
        Amplitude::new(self.0.powf(1.0 / GAMMA))
    }

    fn loudness(&self) -> Loudness {
        *self
    }
}

// azimuth
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A perceived direction on the listener's horizontal plane, in radians.
///
/// Zero is straight ahead, `-π/2` hard left, `π/2` hard right, and `-π`
/// straight back. Values wrap into `[-π, π)`, so `π` folds onto `-π`. The steps
/// are uniform angle, so the scale does not model the finer resolution the ear
/// has near the front.
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

/// the angle states itself both ways.
impl Panned for Azimuth {
    fn azimuth(&self) -> Azimuth {
        *self
    }

    fn axis(&self) -> Axis {
        Axis::new(self.radians.sin())
    }
}

// axis
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A left-right position on the stereo field.
///
/// `-1` is hard left, `0` is center, and `1` is hard right. Front and back fold
/// onto the same value, so an axis cannot be turned back into a full direction
/// without choosing a side.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis(f32);

impl Axis {
    /// hard left.
    pub const LEFT: Self = Self(-1.0);
    /// center.
    pub const CENTER: Self = Self(0.0);
    /// hard right.
    pub const RIGHT: Self = Self(1.0);

    /// clamps into `[-1, 1]`; non-finite input becomes [`CENTER`](Self::CENTER).
    pub fn new(axis: f32) -> Self {
        let axis = match axis.is_finite() {
            true => axis.clamp(-1.0, 1.0),
            false => 0.0,
        };
        Self(axis)
    }

    /// returns the position.
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// the axis states itself both ways, placing the direction in front.
impl Panned for Axis {
    fn azimuth(&self) -> Azimuth {
        Azimuth::new(self.0.asin())
    }

    fn axis(&self) -> Axis {
        *self
    }
}

// gamma
//
// ++++++++++++============++++++++++++============++++++++++++============

/// exponent relating amplitude to perceived ratio, calibrated so that a ten
/// decibel rise doubles the ratio.
const GAMMA: f32 = 0.6;
