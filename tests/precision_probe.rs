//! Diagnostic probe (not an assertion suite): quantify where pivot-delta
//! imprecision comes from on the models-moved fixtures.
//!
//! Hypothesis under test: the ~1e-4 residuals in reported pivot deltas are
//! input quantization — Roblox serializes world-space CFrames as f32, whose
//! step size (ULP) at this scene's coordinates is ~5e-4 — not error added by
//! the (f64) delta math. Method: read the RAW f32 CFrames from both files,
//! compute per-part world deltas in f64 with no library code involved, and
//! compare their spread to the theoretical ULP and their consensus to the
//! tool's reported deltas.
//!
//! Run with: cargo test --release --test precision_probe -- --ignored --nocapture

mod common;
use common::fixture_str;

use rbx_dom_weak::types::{Ref, Variant};
use rbx_dom_weak::WeakDom;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

const BASE: &str = "rc-builds/rc_manually_saved_build.rbxl";
const MOVED: &str = "models-moved/rc_police_station_model_moved_with_internal_models_moved_too.rbxl";

fn load(relative: &str) -> Option<WeakDom> {
    let path = &fixture_str(relative);
    if !Path::new(path).exists() {
        eprintln!("SKIP: fixture {path} not present");
        return None;
    }
    let file = BufReader::new(File::open(path).unwrap());
    Some(rbx_binary::from_reader(file).unwrap())
}

// Minimal f64 rigid math, independent of the library's implementation so the
// probe cannot inherit its errors.
#[derive(Clone, Copy)]
struct Rigid64 {
    r: [[f64; 3]; 3],
    p: [f64; 3],
}

impl Rigid64 {
    fn from_variant(v: &Variant) -> Option<Self> {
        let cf = match v {
            Variant::CFrame(cf) => cf,
            Variant::OptionalCFrame(Some(cf)) => cf,
            _ => return None,
        };
        let row = |v: &rbx_types::Vector3| [v.x as f64, v.y as f64, v.z as f64];
        Some(Rigid64 {
            r: [&cf.orientation.x, &cf.orientation.y, &cf.orientation.z].map(row),
            p: [
                cf.position.x as f64,
                cf.position.y as f64,
                cf.position.z as f64,
            ],
        })
    }

    fn mul(self, other: Rigid64) -> Rigid64 {
        let mut r = [[0.0; 3]; 3];
        for (i, row) in r.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.r[i][k] * other.r[k][j]).sum();
            }
        }
        let rot = |m: &[[f64; 3]; 3], v: [f64; 3]| {
            [0, 1, 2].map(|i| m[i][0] * v[0] + m[i][1] * v[1] + m[i][2] * v[2])
        };
        let rp = rot(&self.r, other.p);
        Rigid64 {
            r,
            p: [self.p[0] + rp[0], self.p[1] + rp[1], self.p[2] + rp[2]],
        }
    }

    fn inverse(self) -> Rigid64 {
        let rt = [
            [self.r[0][0], self.r[1][0], self.r[2][0]],
            [self.r[0][1], self.r[1][1], self.r[2][1]],
            [self.r[0][2], self.r[1][2], self.r[2][2]],
        ];
        let p =
            [0, 1, 2].map(|i| -(rt[i][0] * self.p[0] + rt[i][1] * self.p[1] + rt[i][2] * self.p[2]));
        Rigid64 { r: rt, p }
    }
}

/// name-path -> CFrame for every BasePart-ish instance below `root`,
/// dropping ambiguous (duplicate) paths so matching is exact-or-nothing.
fn part_cframes(dom: &WeakDom, root: Ref, skip_nested_models: bool) -> HashMap<String, Rigid64> {
    let mut map: HashMap<String, Rigid64> = HashMap::new();
    // Sibling index disambiguates duplicate names (stable across saves as
    // long as nothing reorders); skip_names prunes nested boundary subtrees
    // so each population is a single rigid unit.
    fn visit(
        dom: &WeakDom,
        r: Ref,
        prefix: &str,
        skip_nested_models: bool,
        map: &mut HashMap<String, Rigid64>,
    ) {
        let Some(inst) = dom.get_by_ref(r) else { return };
        if let Some(cf) = inst
            .properties
            .get(&"CFrame".into())
            .and_then(Rigid64::from_variant)
        {
            map.insert(prefix.to_string(), cf);
        }
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for &child in inst.children() {
            let Some(child_inst) = dom.get_by_ref(child) else { continue };
            let idx = counts.entry(child_inst.name.as_str()).or_insert(0);
            *idx += 1;
            if skip_nested_models && child_inst.class.as_str() == "Model" {
                continue;
            }
            let child_path = format!("{prefix}/{}#{}", child_inst.name, idx);
            visit(dom, child, &child_path, skip_nested_models, map);
        }
    }
    visit(dom, root, "", skip_nested_models, &mut map);
    map
}

fn find_by_name_chain(dom: &WeakDom, chain: &[&str]) -> Option<Ref> {
    let mut current = dom.root_ref();
    for name in chain {
        let inst = dom.get_by_ref(current)?;
        current = inst
            .children()
            .iter()
            .copied()
            .find(|&c| dom.get_by_ref(c).map(|i| i.name == *name).unwrap_or(false))?;
    }
    Some(current)
}

fn ulp_at(magnitude: f64) -> f64 {
    if magnitude == 0.0 {
        return f64::MIN_POSITIVE;
    }
    2f64.powi(magnitude.abs().log2().floor() as i32 - 23)
}

struct DeltaStats {
    count: usize,
    mean: [f64; 3],
    plain_mean: [f64; 3],
    max_rot_drift: f64,
    max_coord: f64,
    max_dev: f64,
}

/// Per-part world deltas (side * base^-1) for every path present in both,
/// alongside the same deltas with rotation FORCED to identity (plain
/// position difference). The gap between the two isolates rotation-noise
/// amplification: delta translation includes -R_d * base.p, so a ~1e-7
/// rotation wobble times ~7000-stud coordinates fabricates millistuds.
fn delta_stats(
    base: &HashMap<String, Rigid64>,
    side: &HashMap<String, Rigid64>,
) -> Option<DeltaStats> {
    let mut deltas: Vec<[f64; 3]> = Vec::new();
    let mut plain: Vec<[f64; 3]> = Vec::new();
    let mut max_rot_drift: f64 = 0.0;
    let mut max_coord: f64 = 0.0;
    for (path, base_cf) in base {
        let Some(side_cf) = side.get(path) else { continue };
        let d = side_cf.mul(base_cf.inverse());
        deltas.push(d.p);
        plain.push([
            side_cf.p[0] - base_cf.p[0],
            side_cf.p[1] - base_cf.p[1],
            side_cf.p[2] - base_cf.p[2],
        ]);
        for (i, row) in d.r.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                let expected = if i == j { 1.0 } else { 0.0 };
                max_rot_drift = max_rot_drift.max((cell - expected).abs());
            }
        }
        for c in base_cf.p {
            max_coord = max_coord.max(c.abs());
        }
    }
    if deltas.is_empty() {
        return None;
    }
    let n = deltas.len() as f64;
    let mut mean = [0.0; 3];
    for d in &deltas {
        for i in 0..3 {
            mean[i] += d[i] / n;
        }
    }
    let mut max_dev: f64 = 0.0;
    for d in &deltas {
        for i in 0..3 {
            max_dev = max_dev.max((d[i] - mean[i]).abs());
        }
    }
    let mut plain_mean = [0.0; 3];
    for d in &plain {
        for i in 0..3 {
            plain_mean[i] += d[i] / n;
        }
    }
    Some(DeltaStats {
        count: deltas.len(),
        mean,
        plain_mean,
        max_rot_drift,
        max_coord,
        max_dev,
    })
}

fn report(label: &str, stats: &DeltaStats) {
    let ulp = ulp_at(stats.max_coord);
    println!("--- {label} ---");
    println!("  matched parts: {}", stats.count);
    println!(
        "  consensus world delta: ({:.9}, {:.9}, {:.9})",
        stats.mean[0], stats.mean[1], stats.mean[2]
    );
    let nearest: Vec<f64> = stats.mean.iter().map(|m| m.round()).collect();
    println!(
        "  deviation from integer-stud move: ({:+.2e}, {:+.2e}, {:+.2e})",
        stats.mean[0] - nearest[0],
        stats.mean[1] - nearest[1],
        stats.mean[2] - nearest[2]
    );
    println!(
        "  rotation-forced-identity delta: ({:.9}, {:.9}, {:.9})",
        stats.plain_mean[0], stats.plain_mean[1], stats.plain_mean[2]
    );
    println!(
        "  max rotation drift {:.2e} x lever arm {:.0} = {:.2e} predicted fabricated translation",
        stats.max_rot_drift,
        stats.max_coord,
        stats.max_rot_drift * stats.max_coord
    );
    println!(
        "  max |coordinate| {:.0} -> f32 ULP {:.2e}; per-part scatter around consensus: {:.2e} ({:.1} ULPs)",
        stats.max_coord,
        ulp,
        stats.max_dev,
        stats.max_dev / ulp
    );
}

#[test]
#[ignore]
fn probe_models_moved_precision() {
    let (Some(base), Some(moved)) = (load(BASE), load(MOVED)) else {
        return;
    };

    let chain = [
        "Workspace",
        "Map",
        "Buildings",
        "PoliceStation",
        "Rensselaer Law Enforcement Facility",
    ];
    let base_facility = find_by_name_chain(&base, &chain).expect("facility in base");
    let moved_facility = find_by_name_chain(&moved, &chain).expect("facility in moved");

    // Facility DIRECT population: nested boundary subtrees excluded, so this
    // is one rigid unit and its scatter is pure measurement/quantization.
    let base_parts = part_cframes(&base, base_facility, true);
    let moved_parts = part_cframes(&moved, moved_facility, true);
    if let Some(stats) = delta_stats(&base_parts, &moved_parts) {
        report("facility direct parts (nested models excluded)", &stats);
    }

    // Facility pivot delta straight from WorldPivotData.
    for (label, dom, facility) in [("base", &base, base_facility), ("moved", &moved, moved_facility)] {
        if let Some(pivot) = dom
            .get_by_ref(facility)
            .and_then(|i| i.properties.get(&"WorldPivotData".into()))
            .and_then(Rigid64::from_variant)
        {
            println!("  {label} facility pivot: ({:.9}, {:.9}, {:.9})", pivot.p[0], pivot.p[1], pivot.p[2]);
        }
    }

    // Nested boundaries by name chain relative to the facility.
    for (label, sub_chain) in [
        ("nested 'Model'", vec!["Model"]),
        ("nested RFIDDoor/RFID", vec!["RFIDDoor", "RFID"]),
    ] {
        let base_sub = sub_chain
            .iter()
            .fold(Some(base_facility), |r, name| {
                r.and_then(|r| {
                    base.get_by_ref(r).and_then(|inst| {
                        inst.children().iter().copied().find(|&c| {
                            base.get_by_ref(c).map(|i| i.name == *name).unwrap_or(false)
                        })
                    })
                })
            });
        let moved_sub = sub_chain
            .iter()
            .fold(Some(moved_facility), |r, name| {
                r.and_then(|r| {
                    moved.get_by_ref(r).and_then(|inst| {
                        inst.children().iter().copied().find(|&c| {
                            moved
                                .get_by_ref(c)
                                .map(|i| i.name == *name)
                                .unwrap_or(false)
                        })
                    })
                })
            });
        let (Some(base_sub), Some(moved_sub)) = (base_sub, moved_sub) else {
            println!("--- {label}: not found ---");
            continue;
        };
        let b = part_cframes(&base, base_sub, false);
        let m = part_cframes(&moved, moved_sub, false);
        if let Some(stats) = delta_stats(&b, &m) {
            report(label, &stats);
            let mut paths: Vec<&String> = b.keys().collect();
            paths.sort();
            for path in paths.iter().take(3) {
                if let (Some(bc), Some(mc)) = (b.get(*path), m.get(*path)) {
                    println!(
                        "    {path}: base({:.4}, {:.4}, {:.4}) -> moved({:.4}, {:.4}, {:.4})",
                        bc.p[0], bc.p[1], bc.p[2], mc.p[0], mc.p[1], mc.p[2]
                    );
                }
            }
        } else {
            println!("--- {label}: no matched parts ---");
        }
    }
}
