//! generic event maps: anchored event collections.
//!
//! An event is a payload paired with a unique anchor, so one container holds
//! notes today and other kinds of events later. Adapters that move events
//! along the time or layer axis only need the anchors involved to support
//! that axis.

use crate::types::{Index, LayerAnchor, Tick, TimeAnchor};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZero;
use std::ops::{Deref, DerefMut};

// event map
//
// ++++++++++++============++++++++++++============++++++++++++============

/// ordered collection of events, each identified by a unique anchor.
///
/// Entries stay sorted by anchor, so iteration is deterministic.
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

// stream util
//
// ++++++++++++============++++++++++++============++++++++++++============

/// adapters over streams of anchored events.
///
/// Each method applies when the receiver yields anchored events.
pub trait EventStreamExt: Sized {
    /// Rescales ticks from arbitrary tempo (tick/s) to standard game tick (20 t/s).
    ///
    /// Scaling down may drop anchor uniqueness; see
    /// [`rescale_to_tick_rate`](Self::rescale_to_tick_rate).
    fn rescale_to_game_tick<A, E>(self, tempo: f32) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
    {
        self.rescale_to_tick_rate(tempo, 20)
    }

    /// Rescales ticks from arbitrary tempo (tick/s) to redstone tick (10 t/s).
    ///
    /// Scaling down may drop anchor uniqueness; see
    /// [`rescale_to_tick_rate`](Self::rescale_to_tick_rate).
    fn rescale_to_redstone_tick<A, E>(self, tempo: f32) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
    {
        self.rescale_to_tick_rate(tempo, 10)
    }

    /// Rescales ticks from arbitrary tempo (tick/s) to the given target tick rate (t/s).
    ///
    /// The transform stays integral by approximating the factor to `{n, 1/n}`,
    /// and yields a plain `(A, E)` iterator rather than an [`EventMap`] so the
    /// caller can resolve collisions (e.g. via
    /// [`pack_layers`](Self::pack_layers)).
    ///
    /// Scaling down (factor < 1) may collapse several ticks onto one, so anchors
    /// are no longer guaranteed unique; an anchor-keyed collection then overwrites.
    fn rescale_to_tick_rate<A, E>(
        self,
        tempo: f32,
        target_rate: u32,
    ) -> impl Iterator<Item = (A, E)>
    where
        Self: IntoIterator<Item = (A, E)>,
        A: TimeAnchor,
    {
        // tempo outside (0, 30): assume NBS tick == game tick, fold by target/20
        let scale = match (0.0..30.0).contains(&tempo) {
            true => target_rate as f32 / tempo,
            false => target_rate as f32 / 20.0,
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

    /// Packs the stream into anchor `A2`, assigning successive layers to
    /// same-tick events so none is lost.
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

    /// Counterpart of the `split_by_*` methods: recombines several event groups
    /// into one, stacking them vertically in order so their anchors never
    /// collide. Consumes the groups by value and yields a flat event stream.
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

// event map util
//
// ++++++++++++============++++++++++++============++++++++++++============

impl<A: LayerAnchor + Ord, E> EventMap<A, E> {
    /// Groups events into contiguous blocks separated by empty layers.
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

    /// Splits events into groups of `size` layers each.
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
    /// The number of ticks the collection spans from origin: one past the last
    /// anchor's tick, or 0 when empty.
    pub fn tick_count(&self) -> Tick {
        self.keys()
            .map(|anchor| anchor.into_tick())
            .max()
            .map_or(0, |tick| tick + 1)
    }
}

impl<A: LayerAnchor, E> EventMap<A, E> {
    /// The number of layers the collection spans: one past the highest anchor
    /// layer, or 0 when empty.
    pub fn layer_count(&self) -> Index {
        self.keys()
            .map(|anchor| anchor.into_layer())
            .max()
            .map_or(0, |layer| layer + 1)
    }
}
