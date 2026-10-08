//! The mission set of the spike, as plain Rust constructors.
//!
//! Kits follow the progression of audit 3.5 (cycles 3 / 4 / 5 by tier). The numbers
//! (strengths, topology) are the spike's own and were tuned so that the optimal Trace of
//! the Normal variants falls in a playable range (see README).

use crate::model::{
    Difficulty, Family, Loot, Mission, MissionBuilder,
    Program::{
        Backdoor, BruteForce, Cloak, Decrypt, Exploit, Ghost, Overclock, Quantum, Social, Spoof,
        Virus,
    },
};

use Family::{AiGuardian as Ai, Encryption as Enc, Human as Hum, Network as Net};

/// 4 nodes, tier 1, no sentinel: the easy end.
pub fn easy_4(bonus: u8) -> Mission {
    let mut b = MissionBuilder::new("easy_4", 1, Difficulty::Normal);
    b.strength_bonus(bonus);
    let g = b.node("gateway", None, Loot::None);
    let f = b.node("firewall", Some((Net, 2)), Loot::None);
    let a = b.node("archive", Some((Net, 3)), Loot::None);
    let v = b.node("vault", Some((Net, 3)), Loot::Objective);
    b.edge(g, f).edge(f, v).edge(g, a).edge(a, v);
    b.programs(&[BruteForce, Backdoor, Cloak]);
    b.build()
}

/// 4 nodes with a patrol, Intel, a guardian and the four utilities: small enough for the
/// independent brute force in debug builds. Used by the tests.
pub fn tiny_4(bonus: u8) -> Mission {
    tiny_4_with(Difficulty::Normal, bonus)
}

/// [`tiny_4`] under another difficulty preset.
pub fn tiny_4_with(difficulty: Difficulty, bonus: u8) -> Mission {
    let mut b = MissionBuilder::new("tiny_4", 2, difficulty);
    b.strength_bonus(bonus);
    let g = b.node("gateway", None, Loot::None);
    let a = b.node("relay", Some((Net, 2)), Loot::Intel);
    let h = b.node("helpdesk", Some((Hum, 2)), Loot::None);
    let v = b.node("vault", Some((Enc, 2)), Loot::Objective);
    b.edge(g, a).edge(a, h).edge(a, v).edge(g, h);
    b.sentinel(&[a, g, h], None);
    b.program_charges(Social, 1)
        .program_charges(Cloak, 1)
        .program_charges(Spoof, 1);
    b.programs(&[BruteForce]).cycles(2);
    b.build()
}

/// Brute-force companion of [`tiny_4`] covering Ghost and Overclock.
pub fn tiny_ghost() -> Mission {
    let mut b = MissionBuilder::new("tiny_ghost", 2, Difficulty::Normal);
    let g = b.node("gateway", None, Loot::None);
    let a = b.node("relay", Some((Net, 2)), Loot::None);
    let h = b.node("helpdesk", Some((Hum, 2)), Loot::None);
    let v = b.node("vault", Some((Net, 3)), Loot::Objective);
    b.edge(g, a).edge(a, v).edge(g, h).edge(h, v);
    b.sentinel(&[a, h], None);
    b.programs(&[BruteForce, Ghost, Overclock]).cycles(2);
    b.build()
}

/// The 6-node toy of audit 3.3 (two static guardian sentinels), rebuilt in this model.
pub fn audit_6(bonus: u8) -> Mission {
    let mut b = MissionBuilder::new("audit_6", 1, Difficulty::Normal);
    b.strength_bonus(bonus);
    let p = b.node("gateway", None, Loot::None);
    let f = b.node("firewall", Some((Net, 2)), Loot::None);
    let a = b.node("archive", Some((Net, 2)), Loot::None);
    let c = b.node("vault", Some((Net, 4)), Loot::Objective);
    let s = b.node("sentinel", Some((Ai, 2)), Loot::None);
    let k = b.node("cameras", Some((Ai, 3)), Loot::None);
    b.edge(p, f)
        .edge(p, s)
        .edge(f, a)
        .edge(s, a)
        .edge(a, c)
        .edge(a, k)
        .edge(c, k);
    b.sentinel(&[s], Some(s)).sentinel(&[k], Some(k));
    b.programs(&[BruteForce, Exploit, Cloak]);
    b.build()
}

/// 5 nodes, tier 2, one patrol of period 3 around the triangle of the middle layer.
pub fn patrol_5(bonus: u8) -> Mission {
    let mut b = MissionBuilder::new("patrol_5", 2, Difficulty::Normal);
    b.strength_bonus(bonus);
    let g = b.node("gateway", None, Loot::None);
    let w = b.node("proxy", Some((Net, 2)), Loot::None);
    let d = b.node("cipher", Some((Enc, 3)), Loot::None);
    let l = b.node("lan", Some((Net, 2)), Loot::None);
    let v = b.node("vault", Some((Enc, 4)), Loot::Objective);
    b.edge(g, w)
        .edge(g, l)
        .edge(w, d)
        .edge(l, d)
        .edge(w, l)
        .edge(d, v);
    b.sentinel(&[w, d, l], None);
    b.programs(&[BruteForce, Backdoor, Decrypt, Exploit, Spoof, Cloak]);
    b.build()
}

/// 6 nodes, tier 3: three families, Intel to loot before Social engineering works,
/// one patrol of period 4 (ping-pong).
pub fn cipher_6(bonus: u8) -> Mission {
    let mut b = MissionBuilder::new("cipher_6", 3, Difficulty::Normal);
    b.strength_bonus(bonus);
    let g = b.node("gateway", None, Loot::None);
    let f = b.node("firewall", Some((Net, 2)), Loot::None);
    let e = b.node("archive", Some((Enc, 3)), Loot::Intel);
    let x = b.node("relay", Some((Net, 3)), Loot::None);
    let h = b.node("helpdesk", Some((Hum, 4)), Loot::None);
    let v = b.node("vault", Some((Enc, 4)), Loot::Objective);
    b.edge(g, f)
        .edge(f, e)
        .edge(f, x)
        .edge(e, h)
        .edge(x, h)
        .edge(h, v);
    b.sentinel(&[f, e, h, e], None);
    b.programs(&[
        BruteForce, Backdoor, Decrypt, Exploit, Social, Spoof, Cloak, Ghost,
    ]);
    b.build()
}

/// 7 nodes, tier 4, two patrols (periods 3 and 4), a guardian sentinel, Virus and Quantum.
pub fn dual_7(difficulty: Difficulty, bonus: u8) -> Mission {
    let name = format!("dual_7_{}", difficulty.name().to_lowercase());
    let mut b = MissionBuilder::new(&name, 4, difficulty);
    b.strength_bonus(bonus);
    let g = b.node("gateway", None, Loot::None);
    let f = b.node("firewall", Some((Net, 3)), Loot::None);
    let e = b.node("crypto", Some((Enc, 3)), Loot::None);
    let s = b.node("guardian", Some((Ai, 2)), Loot::None);
    let i = b.node("intel", Some((Net, 2)), Loot::Intel);
    let h = b.node("helpdesk", Some((Hum, 3)), Loot::None);
    let v = b.node("vault", Some((Enc, 5)), Loot::Objective);
    b.edge(g, f)
        .edge(g, e)
        .edge(f, e)
        .edge(f, s)
        .edge(e, s)
        .edge(f, i);
    b.edge(e, h).edge(s, v).edge(h, v).edge(i, h);
    b.sentinel(&[g, f, e], None); // patrol of period 3
    b.sentinel(&[e, h, v, h], None); // patrol of period 4
    b.sentinel(&[s], Some(s)); // static guardian process
    b.programs(&[
        BruteForce, Backdoor, Decrypt, Exploit, Quantum, Social, Virus, Spoof, Cloak,
    ]);
    b.build()
}

/// Boss-like site, 8 nodes in three layers: outer firewalls, a core guarded by a
/// sentinel, an inner sanctum. Tier 5.
pub fn boss_8(bonus: u8) -> Mission {
    let mut b = MissionBuilder::new("boss_8", 5, Difficulty::Normal);
    b.strength_bonus(bonus);
    let g = b.node("gateway", None, Loot::None);
    let f1 = b.node("firewall-1", Some((Net, 3)), Loot::Intel);
    let f2 = b.node("firewall-2", Some((Net, 4)), Loot::None);
    let c = b.node("core", Some((Enc, 4)), Loot::None);
    let s = b.node("watchdog", Some((Ai, 3)), Loot::None);
    let h = b.node("operator", Some((Hum, 4)), Loot::None);
    let x = b.node("keystore", Some((Enc, 5)), Loot::None);
    let v = b.node("sanctum", Some((Ai, 4)), Loot::Objective);
    b.edge(g, f1)
        .edge(g, f2)
        .edge(f1, c)
        .edge(f2, c)
        .edge(c, s)
        .edge(c, h)
        .edge(s, h);
    b.edge(s, x).edge(h, x).edge(x, v).edge(h, v);
    b.sentinel(&[c, s, h], None);
    b.sentinel(&[s], Some(s));
    b.programs(&[
        BruteForce, Decrypt, Exploit, Quantum, Social, Virus, Spoof, Cloak, Ghost, Overclock,
    ]);
    b.build()
}

fn worst_graph(name: &str, difficulty: Difficulty, bonus: u8) -> MissionBuilder {
    let mut b = MissionBuilder::new(name, 5, difficulty);
    b.strength_bonus(bonus);
    let g = b.node("gateway", None, Loot::None);
    let a = b.node("firewall", Some((Net, 4)), Loot::None);
    let bb = b.node("proxy", Some((Enc, 4)), Loot::None);
    let c = b.node("helpdesk", Some((Hum, 4)), Loot::None);
    let d = b.node("guardian", Some((Ai, 4)), Loot::None);
    let i = b.node("archive", Some((Net, 3)), Loot::Intel);
    let x = b.node("datastore", Some((Enc, 5)), Loot::Objective);
    let v = b.node("core", Some((Hum, 5)), Loot::Objective);
    b.edge(g, a)
        .edge(a, bb)
        .edge(a, c)
        .edge(bb, c)
        .edge(bb, d)
        .edge(c, d);
    b.edge(d, x).edge(d, v).edge(x, v).edge(a, i).edge(i, bb);
    b.sentinel(&[a, bb, c], None); // patrol of period 3 around the first layer
    b.sentinel(&[d, x, v, x], None); // patrol of period 4 around the inner sanctum
    b.sentinel(&[d], Some(d)); // guardian process of the node d
    b.programs(&[
        BruteForce, Decrypt, Exploit, Quantum, Social, Virus, Spoof, Cloak, Ghost, Overclock,
    ]);
    b
}

/// WORST CASE: 8 nodes, 4 ICE families, 2 patrols (periods 3 and 4) plus a guardian
/// sentinel, 10 programs with charges, 5 cycles (6 in Story), two objectives and Intel.
pub fn worst_8(difficulty: Difficulty, bonus: u8) -> Mission {
    let name = format!("worst_8_{}", difficulty.name().to_lowercase());
    worst_graph(&name, difficulty, bonus).build()
}

/// The worst case with the Backdoor too (11 programs).
pub fn worst_8_full_kit(bonus: u8) -> Mission {
    let mut b = worst_graph("worst_8_kit11", Difficulty::Normal, bonus);
    b.program(Backdoor);
    b.build()
}

/// Unsolvable by deadlock: the only way to the vault is a Human ICE, the kit has no
/// breach program except Social, and the Intel lies behind that very ICE.
pub fn unsolvable_deadlock() -> Mission {
    let mut b = MissionBuilder::new("unsolvable_deadlock", 3, Difficulty::Normal);
    let g = b.node("gateway", None, Loot::None);
    let f = b.node("lan", Some((Net, 2)), Loot::None);
    let h = b.node("helpdesk", Some((Hum, 3)), Loot::None);
    let i = b.node("intel", None, Loot::Intel);
    let v = b.node("vault", Some((Enc, 3)), Loot::Objective);
    b.edge(g, f).edge(f, h).edge(h, i).edge(h, v);
    b.sentinel(&[f, h], None);
    b.programs(&[Social, Cloak, Spoof]);
    b.build()
}

/// Strength bonus applied to each mission of the measured set (tuned, see README).
const B: [u8; 7] = [2, 1, 2, 3, 3, 3, 0];

/// The worst-case graph with every ICE two points stronger and the full kit: still
/// solvable (barely), the heaviest exact search of the set.
pub fn overloaded_8() -> Mission {
    let mut b = worst_graph("overloaded_8", Difficulty::Normal, 2);
    b.cycles(5);
    b.build()
}

/// Unsolvable by Trace: the overloaded graph with only Brute-Force, Decrypt and Cloak.
/// The proof must exhaust every state under the cap, the expensive kind of answer.
pub fn unsolvable_trace() -> Mission {
    let mut b = MissionBuilder::new("unsolvable_trace", 5, Difficulty::Normal);
    let src = overloaded_8();
    b.strength_bonus(0);
    let nodes: Vec<_> = (0..src.n)
        .map(|i| {
            let ice = (src.ice_strength[i] > 0).then_some((src.ice_family[i], src.ice_strength[i]));
            b.node(src.node_names[i], ice, src.loot[i])
        })
        .collect();
    for u in 0..src.n {
        for v in (u + 1)..src.n {
            if src.is_adjacent(u as u8, v as u8) {
                b.edge(nodes[u], nodes[v]);
            }
        }
    }
    for s in &src.sentinels {
        b.sentinel(&s.route, s.anchor);
    }
    b.programs(&[BruteForce, Decrypt, Cloak]);
    b.build()
}

/// The measured set, in order of size.
pub fn all() -> Vec<Mission> {
    vec![
        easy_4(B[0]),
        audit_6(B[1]),
        patrol_5(B[2]),
        cipher_6(B[3]),
        dual_7(Difficulty::Normal, B[4]),
        boss_8(B[5]),
        worst_8(Difficulty::Normal, B[6]),
        worst_8(Difficulty::Story, B[6]),
        overloaded_8(),
        unsolvable_deadlock(),
        unsolvable_trace(),
    ]
}

/// Missions small enough for the debug-mode unit tests.
pub fn small() -> Vec<Mission> {
    vec![tiny_4(0), easy_4(B[0]), audit_6(B[1])]
}

/// Builds a mission of the family `name` with a strength bonus (for scaling experiments).
pub fn by_name(name: &str, bonus: u8) -> Option<Mission> {
    Some(match name {
        "tiny_4" => tiny_4(bonus),
        "tiny_ghost" => tiny_ghost(),
        "easy_4" => easy_4(bonus),
        "audit_6" => audit_6(bonus),
        "patrol_5" => patrol_5(bonus),
        "cipher_6" => cipher_6(bonus),
        "dual_7" => dual_7(Difficulty::Normal, bonus),
        "boss_8" => boss_8(bonus),
        "worst_8" => worst_8(Difficulty::Normal, bonus),
        "worst_8_story" => worst_8(Difficulty::Story, bonus),
        "worst_8_kit11" => worst_8_full_kit(bonus),
        "overloaded_8" => overloaded_8(),
        "unsolvable_trace" => unsolvable_trace(),
        _ => return None,
    })
}

/// The tier's minimal kit: only the programs without charge limit (Brute-Force, Backdoor,
/// Decrypt) that the mission's kit already holds. Invariant I2 asks every mission to be
/// solvable with it.
pub fn minimal_kit(m: &Mission) -> Mission {
    let mut out = m.clone();
    for p in crate::model::Program::ALL {
        if !matches!(p, BruteForce | Backdoor | Decrypt) {
            out.kit[p.index()] = 0;
        }
    }
    out.name = format!("{} (minimal kit)", m.name);
    out
}
