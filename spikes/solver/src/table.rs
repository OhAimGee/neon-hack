//! Deterministic containers for the solver: a `BTreeMap` baseline and a custom
//! open-addressing table with a fixed FxHash-like hash. Neither depends on a random seed
//! (the real engine forbids `HashMap` because of its iteration order).

use std::collections::{BTreeMap, BTreeSet};

/// Result of [`IndexMap::find_or_insert`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Found {
    Inserted,
    Existing(u32),
}

/// Map from a packed state to the index of its node in the arena.
pub trait IndexMap {
    fn new() -> Self;
    /// Looks `key` up. If it is absent it is recorded with index `new_idx`; the caller
    /// pushes the key at that index in `keys` right after. `keys[i]` is the key of node `i`.
    fn find_or_insert(&mut self, key: u128, new_idx: u32, keys: &[u128]) -> Found;
    /// Index of `key` if present.
    fn find(&self, key: u128, keys: &[u128]) -> Option<u32>;
    /// Heap bytes held by the table itself (not by the arena).
    fn heap_bytes(&self) -> usize;
}

/// Set used to merge the states met inside one turn (cleared between expansions).
pub trait LocalSet {
    fn new() -> Self;
    fn clear(&mut self);
    /// Returns true if `key` was not present.
    fn insert(&mut self, key: u128) -> bool;
    fn heap_bytes(&self) -> usize;
}

/// A choice of containers, selected at run time by [`TableKind`].
pub trait Tables {
    type Index: IndexMap;
    type Local: LocalSet;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableKind {
    BTree,
    Fx,
}

impl TableKind {
    pub const fn name(self) -> &'static str {
        match self {
            TableKind::BTree => "btree",
            TableKind::Fx => "fx",
        }
    }
}

pub struct BTreeTables;
impl Tables for BTreeTables {
    type Index = BTreeIndex;
    type Local = BTreeLocal;
}

pub struct FxTables;
impl Tables for FxTables {
    type Index = FxIndex;
    type Local = FxLocal;
}

pub struct BTreeIndex {
    map: BTreeMap<u128, u32>,
}

impl IndexMap for BTreeIndex {
    fn new() -> Self {
        BTreeIndex {
            map: BTreeMap::new(),
        }
    }

    fn find_or_insert(&mut self, key: u128, new_idx: u32, _keys: &[u128]) -> Found {
        use std::collections::btree_map::Entry;
        match self.map.entry(key) {
            Entry::Occupied(e) => Found::Existing(*e.get()),
            Entry::Vacant(v) => {
                v.insert(new_idx);
                Found::Inserted
            }
        }
    }

    fn find(&self, key: u128, _keys: &[u128]) -> Option<u32> {
        self.map.get(&key).copied()
    }

    fn heap_bytes(&self) -> usize {
        // Estimate: a B-tree leaf holds up to 11 pairs of (16 + 4 -> 32 aligned) bytes and
        // is on average ~70 % full, plus the inner nodes: about 1.6 x the payload.
        self.map.len() * 32 * 8 / 5
    }
}

pub struct BTreeLocal {
    set: BTreeSet<u128>,
}

impl LocalSet for BTreeLocal {
    fn new() -> Self {
        BTreeLocal {
            set: BTreeSet::new(),
        }
    }

    fn clear(&mut self) {
        self.set.clear();
    }

    fn insert(&mut self, key: u128) -> bool {
        self.set.insert(key)
    }

    fn heap_bytes(&self) -> usize {
        self.set.len() * 16 * 8 / 5
    }
}

/// FxHash-like mixing of a 128-bit key: fixed multiplier, no seed.
#[inline]
pub fn fx_hash(key: u128) -> u64 {
    const K: u64 = 0x517c_c1b7_2722_0a95;
    let lo = key as u64;
    let hi = (key >> 64) as u64;
    let h = lo.wrapping_mul(K);
    (h.rotate_left(5) ^ hi).wrapping_mul(K)
}

/// Open addressing, linear probing, power-of-two size, load factor <= 1/2. A slot holds
/// `index + 1` into the arena (0 = empty): 4 bytes per slot, keys are compared in the arena.
pub struct FxIndex {
    slots: Vec<u32>,
    shift: u32,
    len: usize,
}

impl FxIndex {
    fn slot_of(&self, key: u128) -> usize {
        (fx_hash(key) >> self.shift) as usize
    }

    fn grow(&mut self, keys: &[u128]) {
        let new_len = (self.slots.len() * 2).max(1024);
        self.shift = 64 - new_len.trailing_zeros();
        self.slots = vec![0; new_len];
        let mask = new_len - 1;
        for (i, &k) in keys.iter().enumerate().take(self.len) {
            let mut s = self.slot_of(k);
            while self.slots[s] != 0 {
                s = (s + 1) & mask;
            }
            self.slots[s] = i as u32 + 1;
        }
    }
}

impl IndexMap for FxIndex {
    fn new() -> Self {
        FxIndex {
            slots: Vec::new(),
            shift: 64,
            len: 0,
        }
    }

    fn find_or_insert(&mut self, key: u128, new_idx: u32, keys: &[u128]) -> Found {
        if (self.len + 1) * 2 > self.slots.len() {
            self.grow(keys);
        }
        let mask = self.slots.len() - 1;
        let mut s = self.slot_of(key);
        loop {
            let v = self.slots[s];
            if v == 0 {
                self.slots[s] = new_idx + 1;
                self.len += 1;
                return Found::Inserted;
            }
            if keys[(v - 1) as usize] == key {
                return Found::Existing(v - 1);
            }
            s = (s + 1) & mask;
        }
    }

    fn find(&self, key: u128, keys: &[u128]) -> Option<u32> {
        if self.slots.is_empty() {
            return None;
        }
        let mask = self.slots.len() - 1;
        let mut s = self.slot_of(key);
        loop {
            let v = self.slots[s];
            if v == 0 {
                return None;
            }
            if keys[(v - 1) as usize] == key {
                return Some(v - 1);
            }
            s = (s + 1) & mask;
        }
    }

    fn heap_bytes(&self) -> usize {
        self.slots.capacity() * 4
    }
}

/// Local set with generation stamps, so that `clear` is O(1).
pub struct FxLocal {
    keys: Vec<u128>,
    stamps: Vec<u32>,
    epoch: u32,
    shift: u32,
    len: usize,
}

impl FxLocal {
    fn grow(&mut self) {
        let new_len = (self.keys.len() * 2).max(256);
        let old_keys = std::mem::take(&mut self.keys);
        let old_stamps = std::mem::take(&mut self.stamps);
        self.keys = vec![0; new_len];
        self.stamps = vec![0; new_len];
        self.shift = 64 - new_len.trailing_zeros();
        let mask = new_len - 1;
        for (k, st) in old_keys.into_iter().zip(old_stamps) {
            if st == self.epoch {
                let mut s = (fx_hash(k) >> self.shift) as usize;
                while self.stamps[s] == self.epoch {
                    s = (s + 1) & mask;
                }
                self.keys[s] = k;
                self.stamps[s] = self.epoch;
            }
        }
    }
}

impl LocalSet for FxLocal {
    fn new() -> Self {
        FxLocal {
            keys: Vec::new(),
            stamps: Vec::new(),
            epoch: 1,
            shift: 64,
            len: 0,
        }
    }

    fn clear(&mut self) {
        self.epoch += 1;
        self.len = 0;
    }

    fn insert(&mut self, key: u128) -> bool {
        if (self.len + 1) * 2 > self.keys.len() {
            self.grow();
        }
        let mask = self.keys.len() - 1;
        let mut s = (fx_hash(key) >> self.shift) as usize;
        while self.stamps[s] == self.epoch {
            if self.keys[s] == key {
                return false;
            }
            s = (s + 1) & mask;
        }
        self.keys[s] = key;
        self.stamps[s] = self.epoch;
        self.len += 1;
        true
    }

    fn heap_bytes(&self) -> usize {
        self.keys.capacity() * 16 + self.stamps.capacity() * 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Pcg32;

    fn check_index<I: IndexMap>() {
        let mut rng = Pcg32::new(7, 1);
        let mut idx = I::new();
        let mut keys: Vec<u128> = Vec::new();
        let mut model: BTreeMap<u128, u32> = BTreeMap::new();
        for _ in 0..20_000 {
            let key = (u128::from(rng.below(5000)) << 70) | u128::from(rng.below(3)) | (1 << 127);
            let new_idx = keys.len() as u32;
            let got = idx.find_or_insert(key, new_idx, &keys);
            match model.get(&key) {
                Some(&i) => assert_eq!(got, Found::Existing(i)),
                None => {
                    assert_eq!(got, Found::Inserted);
                    model.insert(key, new_idx);
                    keys.push(key);
                }
            }
        }
        assert_eq!(model.len(), keys.len());
    }

    #[test]
    fn indexes_agree_with_a_model() {
        check_index::<BTreeIndex>();
        check_index::<FxIndex>();
    }

    fn check_local<L: LocalSet>() {
        let mut rng = Pcg32::new(9, 3);
        let mut set = L::new();
        for _round in 0..50 {
            set.clear();
            let mut model = BTreeSet::new();
            for _ in 0..500 {
                let key = u128::from(rng.below(300)) + 1;
                assert_eq!(set.insert(key), model.insert(key));
            }
        }
    }

    #[test]
    fn local_sets_agree_with_a_model() {
        check_local::<BTreeLocal>();
        check_local::<FxLocal>();
    }
}
