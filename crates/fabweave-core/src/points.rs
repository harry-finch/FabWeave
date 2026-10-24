//! Listes de points calculées en Rust natif (ADR-0002), sur le modèle des
//! fonctions géométriques de FullControl (`circleXY`, `arcXY`, `helixZ`,
//! `move` avec copies…) : on construit une forme, on la transforme, puis on
//! l'imprime d'un coup avec `path.print_points(points, w, h)`.
//!
//! Angles en radians (comme `sin`, `cos`, `print_polar`), sens trigonométrique
//! (antihoraire vu de dessus). Une forme fermée répète son premier point à la
//! fin. Les boucles tournent en Rust : quelques millisecondes pour des
//! centaines de milliers de points, contre ≈ 1,6 µs par point en script.

use rhai::{Dynamic, Engine, FnPtr, NativeCallContext, INT};

use crate::path::MoveKind;
use crate::script::{fault, num, ScriptPath, R};

/// Liste de points (x, y, z), en mm.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Points(pub Vec<[f64; 3]>);

/// Nombre entier positif passé à une fonction (segments, côtés, copies).
fn count(v: &Dynamic, what: &str, min: i64) -> R<usize> {
    let n = num(v, what)?;
    if n.fract() != 0.0 || n < min as f64 || n > 10_000_000.0 {
        return Err(fault("points.invalid_count", format!("{what} : {n}")));
    }
    Ok(n as usize)
}

fn positive(v: &Dynamic, what: &str) -> R<f64> {
    let x = num(v, what)?;
    if !(x > 0.0 && x.is_finite()) {
        return Err(fault("points.invalid_value", format!("{what} : {x}")));
    }
    Ok(x)
}

struct Builder {
    pts: Vec<[f64; 3]>,
    limit: usize,
}

impl Builder {
    fn new(limit: usize, n: usize) -> R<Self> {
        if n > limit {
            return Err(fault("path.too_many_points", limit.to_string()));
        }
        Ok(Self {
            pts: Vec::with_capacity(n),
            limit,
        })
    }

    fn push(&mut self, p: [f64; 3]) -> R<()> {
        if self.pts.len() >= self.limit {
            return Err(fault("path.too_many_points", self.limit.to_string()));
        }
        self.pts.push(p);
        Ok(())
    }

    fn done(self) -> Points {
        Points(self.pts)
    }
}

fn polar(cx: f64, cy: f64, r: f64, a: f64, z: f64) -> [f64; 3] {
    [cx + r * libm::cos(a), cy + r * libm::sin(a), z]
}

type D = Dynamic;

pub(crate) fn register(engine: &mut Engine, limit: usize) {
    engine.register_type_with_name::<Points>("Points");

    // --- Construction ---------------------------------------------------------
    engine.register_fn("points", Points::default);
    engine.register_fn("push", move |p: &mut Points, x: D, y: D, z: D| -> R<()> {
        if p.0.len() >= limit {
            return Err(fault("path.too_many_points", limit.to_string()));
        }
        p.0.push([num(&x, "x")?, num(&y, "y")?, num(&z, "z")?]);
        Ok(())
    });
    engine.register_fn("len", |p: &mut Points| p.0.len() as INT);
    engine.register_fn("point", |p: &mut Points, i: INT| -> R<rhai::Array> {
        let n = p.0.len() as INT;
        let k = if i < 0 { n + i } else { i };
        let q =
            p.0.get(k as usize)
                .filter(|_| k >= 0)
                .ok_or_else(|| fault("points.index_out_of_range", i.to_string()))?;
        Ok(q.iter().map(|v| Dynamic::from_float(*v)).collect())
    });

    // line(x0, y0, z0, x1, y1, z1, segments)
    engine.register_fn(
        "line",
        move |x0: D, y0: D, z0: D, x1: D, y1: D, z1: D, n: D| -> R<Points> {
            let a = [num(&x0, "x0")?, num(&y0, "y0")?, num(&z0, "z0")?];
            let b = [num(&x1, "x1")?, num(&y1, "y1")?, num(&z1, "z1")?];
            let n = count(&n, "segments", 1)?;
            let mut out = Builder::new(limit, n + 1)?;
            for i in 0..=n {
                let t = i as f64 / n as f64;
                out.push([
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ])?;
            }
            Ok(out.done())
        },
    );

    // arc(cx, cy, z, r, start, sweep, segments) : sweep > 0 antihoraire.
    engine.register_fn(
        "arc",
        move |cx: D, cy: D, z: D, r: D, a0: D, sweep: D, n: D| -> R<Points> {
            let (cx, cy, z, r) = (
                num(&cx, "cx")?,
                num(&cy, "cy")?,
                num(&z, "z")?,
                positive(&r, "r")?,
            );
            let (a0, sweep) = (num(&a0, "start")?, num(&sweep, "sweep")?);
            let n = count(&n, "segments", 1)?;
            let mut out = Builder::new(limit, n + 1)?;
            for i in 0..=n {
                out.push(polar(cx, cy, r, a0 + sweep * i as f64 / n as f64, z))?;
            }
            Ok(out.done())
        },
    );

    // circle(cx, cy, z, r, start, segments) : fermé, antihoraire.
    engine.register_fn(
        "circle",
        move |cx: D, cy: D, z: D, r: D, a0: D, n: D| -> R<Points> {
            let (cx, cy, z, r) = (
                num(&cx, "cx")?,
                num(&cy, "cy")?,
                num(&z, "z")?,
                positive(&r, "r")?,
            );
            let (a0, n) = (num(&a0, "start")?, count(&n, "segments", 3)?);
            let mut out = Builder::new(limit, n + 1)?;
            for i in 0..n {
                out.push(polar(
                    cx,
                    cy,
                    r,
                    a0 + std::f64::consts::TAU * i as f64 / n as f64,
                    z,
                ))?;
            }
            let first = out.pts[0];
            out.push(first)?;
            Ok(out.done())
        },
    );

    // polygon(cx, cy, z, r, sides, start) : sommets sur le cercle de rayon r, fermé.
    engine.register_fn(
        "polygon",
        move |cx: D, cy: D, z: D, r: D, sides: D, a0: D| -> R<Points> {
            let (cx, cy, z, r) = (
                num(&cx, "cx")?,
                num(&cy, "cy")?,
                num(&z, "z")?,
                positive(&r, "r")?,
            );
            let (n, a0) = (count(&sides, "sides", 3)?, num(&a0, "start")?);
            let mut out = Builder::new(limit, n + 1)?;
            for i in 0..=n {
                out.push(polar(
                    cx,
                    cy,
                    r,
                    a0 + std::f64::consts::TAU * (i % n) as f64 / n as f64,
                    z,
                ))?;
            }
            Ok(out.done())
        },
    );

    // rectangle(x0, y0, z, dx, dy) : fermé, antihoraire depuis (x0, y0).
    engine.register_fn(
        "rectangle",
        |x0: D, y0: D, z: D, dx: D, dy: D| -> R<Points> {
            let (x, y, z, dx, dy) = (
                num(&x0, "x0")?,
                num(&y0, "y0")?,
                num(&z, "z")?,
                num(&dx, "dx")?,
                num(&dy, "dy")?,
            );
            Ok(Points(vec![
                [x, y, z],
                [x + dx, y, z],
                [x + dx, y + dy, z],
                [x, y + dy, z],
                [x, y, z],
            ]))
        },
    );

    // spiral(cx, cy, z, r_start, r_end, start, turns, segments_per_turn) : à plat.
    engine.register_fn(
        "spiral",
        move |cx: D, cy: D, z: D, r0: D, r1: D, a0: D, turns: D, spt: D| -> R<Points> {
            let (cx, cy, z) = (num(&cx, "cx")?, num(&cy, "cy")?, num(&z, "z")?);
            let (r0, r1, a0, turns) = (
                num(&r0, "r_start")?,
                num(&r1, "r_end")?,
                num(&a0, "start")?,
                positive(&turns, "turns")?,
            );
            let n = (turns * count(&spt, "segments_per_turn", 3)? as f64)
                .round()
                .max(1.0) as usize;
            let mut out = Builder::new(limit, n + 1)?;
            for i in 0..=n {
                let t = i as f64 / n as f64;
                out.push(polar(
                    cx,
                    cy,
                    r0 + (r1 - r0) * t,
                    a0 + std::f64::consts::TAU * turns * t,
                    z,
                ))?;
            }
            Ok(out.done())
        },
    );

    // helix(cx, cy, z_start, r_start, r_end, start, turns, pitch, segments_per_turn) :
    // Z monte de `pitch` par tour, en continu.
    engine.register_fn(
        "helix",
        move |cx: D, cy: D, z0: D, r0: D, r1: D, a0: D, turns: D, pitch: D, spt: D| -> R<Points> {
            let (cx, cy, z0) = (num(&cx, "cx")?, num(&cy, "cy")?, num(&z0, "z_start")?);
            let (r0, r1, a0) = (num(&r0, "r_start")?, num(&r1, "r_end")?, num(&a0, "start")?);
            let (turns, pitch) = (positive(&turns, "turns")?, num(&pitch, "pitch")?);
            let n = (turns * count(&spt, "segments_per_turn", 3)? as f64)
                .round()
                .max(1.0) as usize;
            let mut out = Builder::new(limit, n + 1)?;
            for i in 0..=n {
                let t = i as f64 / n as f64;
                out.push(polar(
                    cx,
                    cy,
                    r0 + (r1 - r0) * t,
                    a0 + std::f64::consts::TAU * turns * t,
                    z0 + pitch * turns * t,
                ))?;
            }
            Ok(out.done())
        },
    );

    // zigzag(x0, y0, z, length, spacing, lines) : lignes parallèles à X, de
    // longueur `length`, espacées de `spacing` en Y, parcourues en aller-retour.
    engine.register_fn(
        "zigzag",
        move |x0: D, y0: D, z: D, len: D, sp: D, lines: D| -> R<Points> {
            let (x0, y0, z, len, sp) = (
                num(&x0, "x0")?,
                num(&y0, "y0")?,
                num(&z, "z")?,
                num(&len, "length")?,
                num(&sp, "spacing")?,
            );
            let n = count(&lines, "lines", 1)?;
            let mut out = Builder::new(limit, 2 * n)?;
            for i in 0..n {
                let y = y0 + sp * i as f64;
                let (a, b) = if i % 2 == 0 {
                    (x0, x0 + len)
                } else {
                    (x0 + len, x0)
                };
                out.push([a, y, z])?;
                out.push([b, y, z])?;
            }
            Ok(out.done())
        },
    );

    // sine(x0, y0, z, length, amplitude, period, segments_per_period) : onde le long de X.
    engine.register_fn(
        "sine",
        move |x0: D, y0: D, z: D, len: D, amp: D, period: D, spp: D| -> R<Points> {
            let (x0, y0, z, len, amp) = (
                num(&x0, "x0")?,
                num(&y0, "y0")?,
                num(&z, "z")?,
                positive(&len, "length")?,
                num(&amp, "amplitude")?,
            );
            let period = positive(&period, "period")?;
            let n = ((len / period) * count(&spp, "segments_per_period", 2)? as f64)
                .round()
                .max(1.0) as usize;
            let mut out = Builder::new(limit, n + 1)?;
            for i in 0..=n {
                let x = len * i as f64 / n as f64;
                out.push([
                    x0 + x,
                    y0 + amp * libm::sin(std::f64::consts::TAU * x / period),
                    z,
                ])?;
            }
            Ok(out.done())
        },
    );

    // --- Transformations (renvoient une nouvelle liste) ---------------------------
    engine.register_fn("+", move |a: Points, b: Points| -> R<Points> {
        if a.0.len() + b.0.len() > limit {
            return Err(fault("path.too_many_points", limit.to_string()));
        }
        let mut v = a.0;
        v.extend(b.0);
        Ok(Points(v))
    });
    engine.register_fn("+=", move |a: &mut Points, b: Points| -> R<()> {
        if a.0.len() + b.0.len() > limit {
            return Err(fault("path.too_many_points", limit.to_string()));
        }
        a.0.extend(b.0);
        Ok(())
    });
    engine.register_fn("reverse", |p: &mut Points| {
        let mut v = p.0.clone();
        v.reverse();
        Points(v)
    });
    engine.register_fn(
        "translate",
        |p: &mut Points, dx: D, dy: D, dz: D| -> R<Points> {
            let (dx, dy, dz) = (num(&dx, "dx")?, num(&dy, "dy")?, num(&dz, "dz")?);
            Ok(Points(
                p.0.iter()
                    .map(|q| [q[0] + dx, q[1] + dy, q[2] + dz])
                    .collect(),
            ))
        },
    );
    engine.register_fn(
        "rotate",
        |p: &mut Points, a: D, cx: D, cy: D| -> R<Points> {
            let (a, cx, cy) = (num(&a, "angle")?, num(&cx, "cx")?, num(&cy, "cy")?);
            let (c, s) = (libm::cos(a), libm::sin(a));
            Ok(Points(
                p.0.iter()
                    .map(|q| {
                        let (x, y) = (q[0] - cx, q[1] - cy);
                        [cx + x * c - y * s, cy + x * s + y * c, q[2]]
                    })
                    .collect(),
            ))
        },
    );
    engine.register_fn("scale", |p: &mut Points, k: D, cx: D, cy: D| -> R<Points> {
        let (k, cx, cy) = (num(&k, "factor")?, num(&cx, "cx")?, num(&cy, "cy")?);
        Ok(Points(
            p.0.iter()
                .map(|q| [cx + (q[0] - cx) * k, cy + (q[1] - cy) * k, q[2]])
                .collect(),
        ))
    });
    engine.register_fn("mirror_x", |p: &mut Points, x0: D| -> R<Points> {
        let x0 = num(&x0, "x")?;
        Ok(Points(
            p.0.iter().map(|q| [2.0 * x0 - q[0], q[1], q[2]]).collect(),
        ))
    });
    engine.register_fn("mirror_y", |p: &mut Points, y0: D| -> R<Points> {
        let y0 = num(&y0, "y")?;
        Ok(Points(
            p.0.iter().map(|q| [q[0], 2.0 * y0 - q[1], q[2]]).collect(),
        ))
    });
    // copies(count, dx, dy, dz) : la liste et ses copies décalées, à la suite.
    engine.register_fn(
        "copies",
        move |p: &mut Points, n: D, dx: D, dy: D, dz: D| -> R<Points> {
            let n = count(&n, "count", 1)?;
            let (dx, dy, dz) = (num(&dx, "dx")?, num(&dy, "dy")?, num(&dz, "dz")?);
            let mut out = Builder::new(limit, p.0.len() * n)?;
            for k in 0..n {
                let k = k as f64;
                for q in &p.0 {
                    out.push([q[0] + dx * k, q[1] + dy * k, q[2] + dz * k])?;
                }
            }
            Ok(out.done())
        },
    );
    // repeat_z(count, dz) : couches empilées (copies décalées en Z).
    engine.register_fn(
        "repeat_z",
        move |p: &mut Points, n: D, dz: D| -> R<Points> {
            let n = count(&n, "count", 1)?;
            let dz = num(&dz, "dz")?;
            let mut out = Builder::new(limit, p.0.len() * n)?;
            for k in 0..n {
                for q in &p.0 {
                    out.push([q[0], q[1], q[2] + dz * k as f64])?;
                }
            }
            Ok(out.done())
        },
    );
    // polar_copies(count, cx, cy) : copies tournées de 2π / count autour du centre.
    engine.register_fn(
        "polar_copies",
        move |p: &mut Points, n: D, cx: D, cy: D| -> R<Points> {
            let n = count(&n, "count", 1)?;
            let (cx, cy) = (num(&cx, "cx")?, num(&cy, "cy")?);
            let mut out = Builder::new(limit, p.0.len() * n)?;
            for k in 0..n {
                let a = std::f64::consts::TAU * k as f64 / n as f64;
                let (c, s) = (libm::cos(a), libm::sin(a));
                for q in &p.0 {
                    let (x, y) = (q[0] - cx, q[1] - cy);
                    out.push([cx + x * c - y * s, cy + x * s + y * c, q[2]])?;
                }
            }
            Ok(out.done())
        },
    );
    // subdivide(max_length) : points ajoutés pour qu'aucun segment ne dépasse max_length.
    engine.register_fn("subdivide", move |p: &mut Points, m: D| -> R<Points> {
        let m = positive(&m, "max_length")?;
        let mut out = Builder::new(limit, p.0.len())?;
        for (i, q) in p.0.iter().enumerate() {
            if i > 0 {
                let a = p.0[i - 1];
                let d =
                    ((q[0] - a[0]).powi(2) + (q[1] - a[1]).powi(2) + (q[2] - a[2]).powi(2)).sqrt();
                let k = (d / m).ceil() as usize;
                for j in 1..k {
                    let t = j as f64 / k as f64;
                    out.push([
                        a[0] + (q[0] - a[0]) * t,
                        a[1] + (q[1] - a[1]) * t,
                        a[2] + (q[2] - a[2]) * t,
                    ])?;
                }
            }
            out.push(*q)?;
        }
        Ok(out.done())
    });
    // modulate_z(f) : z += f(t, x, y, z), t de 0 à 1 le long de la liste.
    engine.register_fn(
        "modulate_z",
        |ctx: NativeCallContext, p: &mut Points, f: FnPtr| -> R<Points> {
            let n = p.0.len();
            let mut v = Vec::with_capacity(n);
            for (i, q) in p.0.iter().enumerate() {
                let t = if n > 1 {
                    i as f64 / (n - 1) as f64
                } else {
                    0.0
                };
                let d: Dynamic = f.call_within_context(&ctx, (t, q[0], q[1], q[2]))?;
                v.push([q[0], q[1], q[2] + num(&d, "modulate_z")?]);
            }
            Ok(Points(v))
        },
    );
    // modulate_r(cx, cy, f) : rayon autour de (cx, cy) += f(t, angle, z).
    engine.register_fn(
        "modulate_r",
        |ctx: NativeCallContext, p: &mut Points, cx: D, cy: D, f: FnPtr| -> R<Points> {
            let (cx, cy) = (num(&cx, "cx")?, num(&cy, "cy")?);
            let n = p.0.len();
            let mut v = Vec::with_capacity(n);
            for (i, q) in p.0.iter().enumerate() {
                let t = if n > 1 {
                    i as f64 / (n - 1) as f64
                } else {
                    0.0
                };
                let (x, y) = (q[0] - cx, q[1] - cy);
                let (r, a) = (x.hypot(y), libm::atan2(y, x));
                let d: Dynamic = f.call_within_context(&ctx, (t, a, q[2]))?;
                let r = r + num(&d, "modulate_r")?;
                v.push([cx + r * libm::cos(a), cy + r * libm::sin(a), q[2]]);
            }
            Ok(Points(v))
        },
    );

    // --- Vers le parcours -------------------------------------------------------------
    engine.register_fn(
        "print_points",
        |path: &mut ScriptPath, p: Points, w: D, h: D| -> R<()> {
            let (w, h) = (num(&w, "w")?, num(&h, "h")?);
            for q in p.0 {
                path.push(q, w, h, MoveKind::Print)?;
            }
            Ok(())
        },
    );
    engine.register_fn(
        "bridge_points",
        |path: &mut ScriptPath, p: Points, w: D, h: D, flow: D| -> R<()> {
            let (w, h, flow) = (num(&w, "w")?, num(&h, "h")?, num(&flow, "flow")?);
            for q in p.0 {
                path.push_bridge(q, w, h, flow)?;
            }
            Ok(())
        },
    );
    engine.register_fn("travel_to", |path: &mut ScriptPath, p: Points| -> R<()> {
        let q = *p.0.first().ok_or_else(|| fault("points.empty", ""))?;
        path.push(q, 0.0, 0.0, MoveKind::Travel)
    });
}
