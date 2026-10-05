//! Newtype identities for bangumi and subtitle groups.
//!
//! These wrap the raw `u32` ids so that an identity can never be confused with
//! another id (or with a plain number), and so signatures document themselves.
//! `#[serde(transparent)]` keeps the serialized form (a bare number) unchanged,
//! so persisted caches / `state.json` need no migration.

use serde::{Deserialize, Serialize};

/// Stable identity of an anime entry.
///
/// A value without a bangumi id is not a Bangumi; this type makes that identity
/// explicit.
///
/// `Default` is a placeholder (`0`) for `..Default::default()` builders only; it is
/// never a real identity.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct BangumiId(u32);

impl BangumiId {
    /// The underlying numeric id.
    pub const fn get(self) -> u32 {
        self.0
    }
}

impl From<u32> for BangumiId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<BangumiId> for u32 {
    fn from(value: BangumiId) -> Self {
        value.0
    }
}

/// Stable identity of a subtitle group.
///
/// A group with no published id is represented by [`SubgroupId::UNKNOWN`] (`0`).
/// Real ids are always non-zero, so `0` is an unambiguous "unknown" sentinel and
/// can be used directly as a key.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct SubgroupId(u32);

impl SubgroupId {
    /// The "unknown subtitle group" sentinel.
    ///
    /// Used when the API data has no real id for a group. Real ids never collide
    /// with it, so it is safe as a map key and for equality checks.
    pub const UNKNOWN: SubgroupId = SubgroupId(0);

    /// The underlying numeric id.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Whether this is a real group id (i.e. not [`SubgroupId::UNKNOWN`]).
    pub const fn is_known(self) -> bool {
        self.0 != 0
    }
}

impl From<u32> for SubgroupId {
    fn from(value: u32) -> Self {
        Self(value)
    }
}

impl From<SubgroupId> for u32 {
    fn from(value: SubgroupId) -> Self {
        value.0
    }
}

/// A specific subtitle group offering of a bangumi: the `(bangumi, subgroup)`
/// pair that subscriptions and lazy episode loading are keyed by.
///
/// Replaces the anonymous `(u32, u32)` tuples that were previously threaded
/// around, so the two ids can't be swapped by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubgroupRef {
    pub bangumi: BangumiId,
    pub subgroup: SubgroupId,
}

impl SubgroupRef {
    pub const fn new(bangumi: BangumiId, subgroup: SubgroupId) -> Self {
        Self { bangumi, subgroup }
    }
}

impl From<(BangumiId, SubgroupId)> for SubgroupRef {
    fn from((bangumi, subgroup): (BangumiId, SubgroupId)) -> Self {
        Self { bangumi, subgroup }
    }
}
