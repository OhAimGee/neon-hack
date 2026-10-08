//! Admissible lower bound on the final Trace, for A*.
//!
//! The cost of a state is not path dependent (the Trace is part of the state), so the search
//! is a best-first exploration ordered by `f(s) = trace - 10 * spoof_charges + h(s)`:
//!
//! * `trace - 10 * spoof_left` never decreases along a transition (a Spoof lowers the Trace
//!   by at most 10 and spends one charge), so it is a valid lower bound on the final Trace
//!   and turns the negative edge of Spoof into a plain uniform-cost search;
//! * `h` adds what any plan must still pay: the noise of the ICE that stand between the
//!   player and each remaining objective (cheapest noise per unit of power in the kit), and
//!   the ambient Trace of the extra turns that the cycles needed will take. A Ghost
//!   charge can cancel one turn of noise, which is subtracted.
//!
//! Scans, the danger-band extra noise and the loss from overkill are ignored: they can only
//! add Trace, so the bound stays admissible.

use std::cell::RefCell;

use crate::model::{FAMILY_COUNT, MAX_NODES, Mission, Program, State, Status, UNLIMITED};

/// "Impossible": the state cannot reach the objectives with this kit.
pub const INFEASIBLE: i32 = i32::MAX / 4;

#[derive(Clone, Debug)]
pub struct Heuristic {
    ghost_turn_noise: i32,
    /// `noise_tab[family][remaining]`: lower bound of the noise to clear `remaining` units.
    noise_tab: [[i32; 7]; FAMILY_COUNT],
    cycle_tab: [[i32; 7]; FAMILY_COUNT],
    /// Directed arcs of the graph.
    arcs: Vec<(u8, u8)>,
    /// Direct-mapped memo of [`Heuristic::path_needs`]: `(key, noise, cycles)`.
    cache: RefCell<Vec<(u64, i32, i32)>>,
    ambient: i32,
    per_turn: i32,
    has_attack: bool,
}

fn less(a: (u32, u32), b: (u32, u32)) -> bool {
    u64::from(a.0) * u64::from(b.1) < u64::from(b.0) * u64::from(a.1)
}

fn ceil_mul(rem: u32, r: (u32, u32)) -> i32 {
    i32::try_from((rem * r.0).div_ceil(r.1)).unwrap_or(INFEASIBLE)
}

impl Heuristic {
    pub fn new(m: &Mission) -> Self {
        let mut noise_ratio = [(u32::MAX, 1); FAMILY_COUNT];
        let mut cycle_ratio = [(u32::MAX, 1); FAMILY_COUNT];
        let mut has_attack = false;
        for p in Program::ALL {
            if !p.is_breach() || m.kit[p.index()] == 0 {
                continue;
            }
            has_attack = true;
            let spec = p.spec();
            for fam in crate::model::Family::ALL {
                let power = u32::from(p.power_against(fam));
                let nr = (u32::from(spec.noise) * u32::from(m.gain_q4), 4 * power);
                if less(nr, noise_ratio[fam.index()]) || noise_ratio[fam.index()].0 == u32::MAX {
                    noise_ratio[fam.index()] = nr;
                }
                let cr = (u32::from(spec.cycles), power);
                if less(cr, cycle_ratio[fam.index()]) || cycle_ratio[fam.index()].0 == u32::MAX {
                    cycle_ratio[fam.index()] = cr;
                }
            }
        }
        // A Ghost turn cancels the noise of the actions that follow the Ghost: at most
        // (cycles - 1) cycles of breach work, i.e. that many units of progress, each of
        // them counted at most at the dearest best-ratio of the families in the bound.
        let capacity = u32::from(m.cycles_per_turn) + if m.has(Program::Overclock) { 2 } else { 0 };
        let mut max_ratio = (0u32, 1u32);
        let mut max_power_per_cycle = (0u32, 1u32);
        for fam in crate::model::Family::ALL {
            let r = noise_ratio[fam.index()];
            if r.0 != u32::MAX && less(max_ratio, r) {
                max_ratio = r;
            }
        }
        for p in Program::ALL {
            if !p.is_breach() || m.kit[p.index()] == 0 {
                continue;
            }
            for fam in crate::model::Family::ALL {
                let pc = (u32::from(p.power_against(fam)), u32::from(p.spec().cycles));
                if less(max_power_per_cycle, pc) {
                    max_power_per_cycle = pc;
                }
            }
        }
        // credit = ceil(units * max_ratio) with units = (capacity - 1) * max_power_per_cycle
        let units_num = capacity.saturating_sub(1) * max_power_per_cycle.0;
        let credit_num = units_num * max_ratio.0;
        let credit_den = max_power_per_cycle.1 * max_ratio.1;
        let ghost = i32::try_from(credit_num.div_ceil(credit_den.max(1))).unwrap_or(INFEASIBLE);
        let mut noise_tab = [[0i32; 7]; FAMILY_COUNT];
        let mut cycle_tab = [[0i32; 7]; FAMILY_COUNT];
        if has_attack {
            for f in 0..FAMILY_COUNT {
                for rem in 1..7u32 {
                    noise_tab[f][rem as usize] = ceil_mul(rem, noise_ratio[f]);
                    cycle_tab[f][rem as usize] = ceil_mul(rem, cycle_ratio[f]);
                }
            }
        }
        let mut arcs = Vec::new();
        for u in 0..m.n {
            for v in 0..m.n {
                if m.adj[u] & (1 << v) != 0 {
                    arcs.push((u as u8, v as u8));
                }
            }
        }
        Heuristic {
            noise_tab,
            cycle_tab,
            arcs,
            cache: RefCell::new(vec![(0, 0, 0); 1 << 16]),
            ghost_turn_noise: ghost,
            ambient: i32::from(m.gain(m.ambient)),
            per_turn: i32::from(m.cycles_per_turn),
            has_attack,
        }
    }

    /// Noise and cycles needed to reach every remaining objective, cheapest path: the part
    /// of the bound that depends on position, progress and loot only.
    fn path_needs(&self, m: &Mission, s: &State, remaining: u8) -> (i32, i32) {
        // Node weights: noise bound and cycle bound to clear the node and walk in.
        let mut wn = [0i32; MAX_NODES];
        let mut wc = [1i32; MAX_NODES];
        for i in 0..m.n {
            let rem = usize::from(m.ice_strength[i].saturating_sub(s.progress[i]));
            if rem > 0 {
                if !self.has_attack {
                    return (INFEASIBLE, INFEASIBLE);
                }
                let f = m.ice_family[i].index();
                wn[i] = self.noise_tab[f][rem];
                wc[i] += self.cycle_tab[f][rem];
            }
        }
        let mut dn = [INFEASIBLE; MAX_NODES];
        let mut dc = [INFEASIBLE; MAX_NODES];
        dn[usize::from(s.pos)] = 0;
        dc[usize::from(s.pos)] = 0;
        loop {
            let mut changed = false;
            for &(u, v) in &self.arcs {
                let (u, v) = (usize::from(u), usize::from(v));
                if dn[u] < INFEASIBLE && dn[u] + wn[v] < dn[v] {
                    dn[v] = dn[u] + wn[v];
                    changed = true;
                }
                if dc[u] < INFEASIBLE && dc[u] + wc[v] < dc[v] {
                    dc[v] = dc[u] + wc[v];
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        let mut noise_need = 0;
        let mut cycle_need = 0;
        for o in 0..m.n {
            if remaining & (1 << o) == 0 {
                continue;
            }
            if dn[o] >= INFEASIBLE {
                return (INFEASIBLE, INFEASIBLE);
            }
            noise_need = noise_need.max(dn[o]);
            cycle_need = cycle_need.max(dc[o]);
        }
        (noise_need, cycle_need)
    }

    /// Lower bound of what remains to pay, or [`INFEASIBLE`].
    pub fn h(&self, m: &Mission, s: &State) -> i32 {
        self.h_and_distance(m, s).0
    }

    /// `(h, distance)`: `distance` is the uncredited sum of the noise and cycles still
    /// needed, a measure of how far the state is from the goal (used to break ties).
    pub fn h_and_distance(&self, m: &Mission, s: &State) -> (i32, i32) {
        if s.status == Status::Won {
            return (0, 0);
        }
        let remaining = m.objective_mask & !s.loot_taken;
        if remaining == 0 {
            return (0, 0);
        }
        // The path part depends on the position, the progress and the loot only: memoised.
        let mut key = (1u64 << 63) | u64::from(s.pos) | (u64::from(s.loot_taken) << 3);
        for (i, &p) in s.progress.iter().enumerate() {
            key |= u64::from(p) << (11 + 3 * i);
        }
        let slot = (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 48) as usize;
        let cached = {
            let c = self.cache.borrow();
            let e = c[slot];
            (e.0 == key).then_some((e.1, e.2))
        };
        let (noise_need, cycle_need) = match cached {
            Some(v) => v,
            None => {
                let v = self.path_needs(m, s, remaining);
                self.cache.borrow_mut()[slot] = (key, v.0, v.1);
                v
            }
        };
        if noise_need >= INFEASIBLE {
            return (INFEASIBLE, INFEASIBLE);
        }
        let mut cycle_need = cycle_need;
        // One Loot action per remaining objective.
        cycle_need += remaining.count_ones() as i32;
        let ocl = if m.kit[Program::Overclock.index()] == 0 {
            0
        } else {
            i32::from(s.charges[Program::Overclock.index()].min(3))
        };
        let capacity = i32::from(s.cycles) + 2 * ocl;
        let per_turn = self.per_turn.max(1);
        let extra_turns = ((cycle_need - capacity).max(0) + per_turn - 1) / per_turn;
        let ambient = extra_turns * self.ambient;
        let ghosts = if m.kit[Program::Ghost.index()] == 0 {
            0
        } else {
            i32::from(s.charges[Program::Ghost.index()].min(3)) + i32::from(s.ghost)
        };
        let h = (noise_need + ambient - ghosts * self.ghost_turn_noise).max(0);
        (h, noise_need + ambient + cycle_need)
    }

    /// `(base, h, distance)`; see [`Heuristic::parts`] and [`Heuristic::h_and_distance`].
    pub fn parts3(&self, m: &Mission, s: &State, use_h: bool) -> (i32, i32, i32) {
        let (base, _) = self.parts(m, s, false);
        if !use_h {
            return (base, 0, 0);
        }
        let (h, d) = self.h_and_distance(m, s);
        (base, h, d)
    }

    /// `(base, h)` with `base = trace - 10 * spoof_charges`; `h` is [`INFEASIBLE`] if hopeless.
    pub fn parts(&self, m: &Mission, s: &State, use_h: bool) -> (i32, i32) {
        let k = m.kit[Program::Spoof.index()];
        let spoof = if k == 0 || k == UNLIMITED {
            0
        } else {
            i32::from(s.charges[Program::Spoof.index()].min(3))
        };
        let base = i32::from(s.trace) - 10 * spoof;
        (base, if use_h { self.h(m, s) } else { 0 })
    }

    /// `f = max(0, base + h)`; [`INFEASIBLE`] if hopeless.
    pub fn f(&self, m: &Mission, s: &State, use_h: bool) -> i32 {
        let (base, h) = self.parts(m, s, use_h);
        if h >= INFEASIBLE {
            INFEASIBLE
        } else {
            (base + h).max(0)
        }
    }
}
