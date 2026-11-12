//! Anchors, event containers, and adapters that move events along an axis.
//!
//! An event is a payload paired with a unique anchor, so one container holds
//! notes today and other kinds of events later. Adapters that move events
//! along the time or layer axis only need the anchors involved to support
//! that axis.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZero;
use std::ops::{Deref, DerefMut};

// anchors
//
// ++++++++++++============++++++++++++============++++++++++++============

/// An event key that carries a point in time.
///
/// Adapters that only move events along the time axis accept any such key, so
/// bare ticks and full positions are interchangeable. Rewriting the tick must
/// leave every other coordinate untouched.
pub trait TimeAnchor: Sized + Copy {
    /// returns the tick coordinate.
    fn into_tick(self) -> Tick;
    /// returns a copy with the tick coordinate replaced.
    fn with_tick(self, tick: Tick) -> Self;
}

impl TimeAnchor for Tick {
    fn into_tick(self) -> Tick {
        self
    }
    fn with_tick(self, tick: Tick) -> Self {
        tick
    }
}

/// An event key that carries a layer.
///
/// Adapters that only move events across layers accept any such key. Rewriting
/// the layer must leave every other coordinate untouched.
pub trait LayerAnchor: Sized + Copy {
    /// returns the layer index.
    fn into_layer(self) -> Index;
    /// returns a copy with the layer index replaced.
    fn with_layer(self, layer: Index) -> Self;
}

impl LayerAnchor for Index {
    fn into_layer(self) -> Index {
        self
    }
    fn with_layer(self, layer: Index) -> Self {
        layer
    }
}

/// Identifies one layer among the layers a score declares.
///
/// Layer numbers are not pitches or volumes; they only group events that can
/// sound at the same moment.
pub type Index = u32;

/// Counts time steps in the score's own tick rate.
///
/// A tick is a position on the time axis, not a duration in seconds; the rate
/// that turns ticks into time belongs to the score's tempo.
pub type Tick = u32;

// event map
//
// ++++++++++++============++++++++++++============++++++++++++============

/// A set of events, each identified by a unique anchor.
///
/// Entries are always ordered by anchor, so iteration is deterministic and
/// inserting the same anchor twice replaces the earlier event.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct EventMap<Anchor, Event>(BTreeMap<Anchor, Event>);

impl<A, E> Default for EventMap<A, E> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<A, E> From<BTreeMap<A, E>> for EventMap<A, E> {
    fn from(map: BTreeMap<A, E>) -> Self {
        Self(map)
    }
}

impl<A: Ord, E> FromIterator<(A, E)> for EventMap<A, E> {
    fn from_iter<I: IntoIterator<Item = (A, E)>>(iter: I) -> Self {
        Self(BTreeMap::from_iter(iter))
    }
}

impl<A: Ord, E> Extend<(A, E)> for EventMap<A, E> {
    fn extend<I: IntoIterator<Item = (A, E)>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<A, E> Deref for EventMap<A, E> {
    type Target = BTreeMap<A, E>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A, E> DerefMut for EventMap<A, E> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<A, E> IntoIterator for EventMap<A, E> {
    type Item = (A, E);
    type IntoIter = std::collections::btree_map::IntoIter<A, E>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, A, E> IntoIterator for &'a EventMap<A, E> {
    type Item = (&'a A, &'a E);
    type IntoIter = std::collections::btree_map::Iter<'a, A, E>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<A: LayerAnchor + Ord, E> EventMap<A, E> {
    /// Cuts the map wherever a layer is unused.
    ///
    /// Each block keeps its events and its original internal layer spacing.
    pub fn split_by_layer_gaps(self) -> Vec<EventMap<A, E>> {
        let layers: BTreeSet<Index> = self.keys().map(|pos| pos.into_layer()).collect();
        let mut starts = Vec::new();
        let mut prev: Option<Index> = None;
        for layer in layers {
            starts.extend(prev.is_none_or(|p| p + 1 != layer).then_some(layer));
            prev = Some(layer);
        }

        let mut groups: Vec<EventMap<A, E>> = Vec::new();
        groups.resize_with(starts.len(), EventMap::default);
        for (pos, event) in self {
            let layer = pos.into_layer();
            let group = starts.partition_point(|&s| s <= layer) - 1;
            groups[group].insert(pos.with_layer(layer - starts[group]), event);
        }
        groups
    }

    /// Cuts the map into consecutive blocks of `size` layers.
    ///
    /// `None` leaves the map whole, and the last block may be shorter.
    pub fn split_by_layer_count(self, size: Option<NonZero<usize>>) -> Vec<EventMap<A, E>> {
        let Some(size) = size.map(|s| s.get() as Index) else {
            return vec![self];
        };
        let mut groups: BTreeMap<Index, BTreeMap<A, E>> = BTreeMap::new();
        for (pos, event) in self {
            let layer = pos.into_layer();
            let entry = groups.entry(layer / size).or_default();
            entry.insert(pos.with_layer(layer % size), event);
        }
        groups.into_values().map(EventMap::from).collect()
    }
}

impl<A: TimeAnchor, E> EventMap<A, E> {
    /// How many ticks the map spans from the origin.
    ///
    /// This is one past the last tick in use, and zero when the map is empty.
    pub fn tick_count(&self) -> Tick {
        self.keys()
            .map(|anchor| anchor.into_tick())
            .max()
            .map_or(0, |tick| tick + 1)
    }
}

impl<A: LayerAnchor, E> EventMap<A, E> {
    /// How many layers the map spans from the bottom.
    ///
    /// This is one past the highest layer in use, and zero when the map is
    /// empty.
    pub fn layer_count(&self) -> Index {
        self.keys()
            .map(|anchor| anchor.into_layer())
            .max()
            .map_or(0, |layer| layer + 1)
    }
}

// stream util
//
// ++++++++++++============++++++++++++============++++++++++++============

/// Adapters for streams of anchored events.
///
/// Every method is available on any iterator whose items pair an anchor with an
/// event; a method only requires the anchor capability it actually uses.
pub trait EventStreamExt: Sized {
    /// Moves events onto a different tick rate, given the source tempo.
    ///
    /// The result is a plain iterator rather than an [`EventMap`], because a
    /// change of rate can map several source ticks to one and the caller is the
    /// one that can still separate them (for example via
    /// [`pack_layers`](Self::pack_layers)). Speeding up may therefore produce
    /// duplicate anchors, which cannot coexist in an anchor-keyed map.
    fn rescale_to_tick_rate<A, E>(
        self,
        tempo: f32,
        target_rate: u32,
    ) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
    {
        // tempo outside (0, 30): assume events already run at the target rate
        let scale = match (0.0..30.0).contains(&tempo) {
            true => target_rate as f32 / tempo,
            false => 1.0,
        };
        // approximate scale to {z, 1/z} as (num, den), keeping tick transforms integral
        let (num, den) = match scale >= 1.0 {
            true => (scale.round() as u32, 1),
            false => (1, (1.0 / scale).round() as u32),
        };
        self.into_iter().map(move |(anchor, event)| {
            let tick = anchor.into_tick() * num / den;
            (anchor.with_tick(tick), event)
        })
    }

    /// Rewrites anchors into `A2`, spreading events that share a tick over
    /// successive layers.
    ///
    /// No event is dropped, and the output order follows the input order.
    fn pack_layers<A, E, A2>(self) -> impl Iterator<Item = (A2, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
        A2: TimeAnchor + LayerAnchor + Default,
    {
        let mut layers: BTreeMap<Tick, Index> = BTreeMap::new();
        self.into_iter().map(move |(anchor, event)| {
            let tick = anchor.into_tick();
            let layer = layers.entry(tick).or_default();
            let pos = A2::default().with_tick(tick).with_layer(*layer);
            *layer += 1;
            (pos, event)
        })
    }

    /// Undoes the `split_by_*` methods by stacking groups back together.
    ///
    /// Groups keep their given order and are offset so anchors never collide.
    fn concat<A, E>(self) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = EventMap<A, E>>,
        A: LayerAnchor,
    {
        let stacked = self.into_iter().scan(0, |offset, n| {
            let base = *offset;
            let f = move |(pos, event): (A, E)| (pos.with_layer(pos.into_layer() + base), event);
            *offset += n.keys().map(|p| p.into_layer()).max().map_or(0, |m| m + 2);
            Some(n.into_iter().map(f))
        });
        stacked.flatten()
    }
}

/// Applies to every type; each method activates when its `Self` bounds hold.
impl<T> EventStreamExt for T {}
