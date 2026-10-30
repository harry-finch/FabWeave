//! Garde-buse (SAFE-03) : la buse et la tête touchent-elles la matière déjà
//! posée ? Une carte des hauteurs est tenue à jour le long du parcours ; à
//! chaque position de la buse, on y cherche de la matière plus haute que ce
//! que sa forme laisse passer :
//!
//! - **bout** : aucune matière au-dessus de la pointe, sous son méplat (le
//!   dessus plat du boudin voisin compris) ;
//! - **cône** (`nozzle.cone_angle`, `cone_height`, si le profil les donne) :
//!   à la distance `d` du méplat, la matière peut monter jusqu'à
//!   `d × tan(angle)` au-dessus de la pointe ;
//! - **tête** (`nozzle.clearance_radius`, `clearance_height`) : rien plus haut
//!   que l'espace libre sous la tête, dans son rayon.
//!
//! Une marge d'une hauteur de boudin est toujours laissée (le plastique chaud
//! s'écarte). Au-delà, la buse **frôle** tant que l'excès reste sous deux
//! hauteurs de boudin (avertissement, compté) ; plus haut, elle **heurte**
//! (erreur, mouvement surligné, export refusé). La tête n'est jamais tolérée.
//!
//! Le dépôt récent (deux largeurs de boudin en arrière) n'entre dans la carte
//! qu'une fois la buse passée : elle ne se heurte pas à ce qu'elle pose. Un
//! parcours dont Z ne redescend jamais est écarté d'emblée : ce test ne coûte
//! rien aux couches planes et aux spirales.
//!
//! Hypothèses et limites : ADR-0008. La carte ne voit que le dessus de la
//! matière (2,5D) et la matière ne s'affaisse pas.

use crate::issue::Issue;
use crate::path::{MoveKind, Toolpath, WayPoint};
use crate::plan::Plan;
use crate::profile::{FilamentProfile, PrinterProfile};

/// Maille la plus fine de la carte (mm).
const MIN_CELL: f64 = 0.25;
/// Plus grand côté de la carte (mailles).
const MAX_CELLS: f64 = 2048.0;
/// Côté d'une tuile (mailles) : le maximum de chaque tuile évite de parcourir
/// les mailles vides.
const TILE: usize = 16;
/// Écart entre deux positions de la buse le long d'un mouvement (mm).
const STEP: f64 = 1.0;
/// Portée de la recherche quand le cône est inconnu (mm), pour dire que le
/// profil manque d'une valeur utile.
const UNKNOWN_CONE_REACH: f64 = 5.0;
/// Marge la plus faible au-dessus de la pointe (mm).
const MIN_MARGIN: f64 = 0.05;
/// Excès toléré au-dessus de la marge avant le heurt, en marges.
const RUB_MARGINS: f64 = 2.0;

/// Carte des hauteurs : 0 = plateau, donc rien à heurter.
pub(crate) struct HeightMap {
    origin: [f64; 2],
    cell: f64,
    size: [usize; 2],
    cells: Vec<f32>,
    tiles_size: [usize; 2],
    tiles: Vec<f32>,
    top: f64,
}

impl HeightMap {
    pub(crate) fn new(min: [f64; 2], max: [f64; 2]) -> Self {
        let span = (max[0] - min[0]).max(max[1] - min[1]);
        let cell = MIN_CELL.max(span / MAX_CELLS);
        let n = |lo: f64, hi: f64| ((hi - lo) / cell).ceil() as usize + 1;
        let size = [n(min[0], max[0]), n(min[1], max[1])];
        let tiles_size = [size[0] / TILE + 1, size[1] / TILE + 1];
        Self {
            origin: min,
            cell,
            size,
            cells: vec![0.0; size[0] * size[1]],
            tiles_size,
            tiles: vec![0.0; tiles_size[0] * tiles_size[1]],
            top: 0.0,
        }
    }

    pub(crate) fn cell(&self) -> f64 {
        self.cell
    }

    /// Verse dans la carte le boudin qui arrive en `q` depuis `p`.
    pub(crate) fn stamp_segment(&mut self, p: &WayPoint, q: &WayPoint) {
        if !q.kind.deposits() {
            return;
        }
        let len = (q.x - p.x).hypot(q.y - p.y);
        let steps = (len / self.cell).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            self.stamp(
                p.x + (q.x - p.x) * t,
                p.y + (q.y - p.y) * t,
                p.z + (q.z - p.z) * t,
            );
        }
    }

    fn stamp(&mut self, x: f64, y: f64, z: f64) {
        let i = (((x - self.origin[0]) / self.cell) as usize).min(self.size[0] - 1);
        let j = (((y - self.origin[1]) / self.cell) as usize).min(self.size[1] - 1);
        let v = z as f32;
        let c = &mut self.cells[j * self.size[0] + i];
        if v > *c {
            *c = v;
            let t = &mut self.tiles[(j / TILE) * self.tiles_size[0] + i / TILE];
            *t = t.max(v);
            self.top = self.top.max(z);
        }
    }

    /// Appelle `f(distance, hauteur)` pour chaque maille plus haute que
    /// `above`, à moins de `reach` de (x, y).
    pub(crate) fn scan(&self, x: f64, y: f64, reach: f64, above: f64, mut f: impl FnMut(f64, f64)) {
        let (fx, fy) = (
            (x - self.origin[0]) / self.cell,
            (y - self.origin[1]) / self.cell,
        );
        let r = reach / self.cell;
        let range = |c: f64, n: usize| {
            let lo = ((c - r).floor().max(0.0) as usize).min(n);
            let hi = ((c + r).ceil().max(0.0) as usize + 1).min(n);
            (lo, hi)
        };
        let (i0, i1) = range(fx, self.size[0]);
        let (j0, j1) = range(fy, self.size[1]);
        if i0 >= i1 || j0 >= j1 {
            return;
        }
        let above = above as f32;
        for tj in j0 / TILE..=(j1 - 1) / TILE {
            for ti in i0 / TILE..=(i1 - 1) / TILE {
                if self.tiles[tj * self.tiles_size[0] + ti] <= above {
                    continue;
                }
                for j in (tj * TILE).max(j0)..((tj + 1) * TILE).min(j1) {
                    let row = j * self.size[0];
                    for i in (ti * TILE).max(i0)..((ti + 1) * TILE).min(i1) {
                        let h = self.cells[row + i];
                        if h > above {
                            let d = (i as f64 + 0.5 - fx).hypot(j as f64 + 0.5 - fy) * self.cell;
                            if d <= reach {
                                f(d, f64::from(h));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Résultat, pour les messages.
#[derive(Default)]
struct Tally {
    nozzle: usize,
    nozzle_excess: f64,
    rub: usize,
    rub_excess: f64,
    head: usize,
    head_excess: f64,
    unknown_cone: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Contact {
    None,
    Rub,
    Hit,
}

/// Forme de la buse et de la tête lue dans le profil.
struct Shape {
    tip: f64,
    cone: Option<(f64, f64)>,
    head: Option<(f64, f64)>,
}

impl Shape {
    /// Teste une position de la buse. `shoulder` : demi-largeur du dessus
    /// plat des boudins voisins (la carte ne porte que leur axe).
    fn test(
        &self,
        map: &HeightMap,
        [x, y, z]: [f64; 3],
        shoulder: f64,
        margin: f64,
        tally: &mut Tally,
    ) -> Contact {
        let level = z + margin;
        let mut worst = 0.0_f64;
        let mut contact = Contact::None;
        if level < map.top {
            let flat = self.tip + shoulder;
            let (reach, tan_a, cone_h) = match self.cone {
                Some((tan_a, h)) => (flat + h / tan_a, tan_a, h),
                None => (flat + UNKNOWN_CONE_REACH, 0.0, 0.0),
            };
            map.scan(x, y, reach, level, |d, height| {
                let allowed = ((d - flat).max(0.0) * tan_a).min(cone_h);
                let excess = height - level - allowed;
                if self.cone.is_none() && d > flat {
                    tally.unknown_cone = true;
                } else {
                    worst = worst.max(excess);
                }
            });
            if worst > RUB_MARGINS * margin {
                contact = Contact::Hit;
                tally.nozzle += 1;
                tally.nozzle_excess = tally.nozzle_excess.max(worst);
            } else if worst > 0.0 {
                contact = Contact::Rub;
                tally.rub_excess = tally.rub_excess.max(worst);
            }
        }
        if let Some((radius, height)) = self.head {
            let level = z + height + margin;
            if level < map.top {
                let mut touched = false;
                // Une maille d'écart : des copies posées exactement à la
                // distance de dégagement ne se touchent pas.
                map.scan(x, y, radius - map.cell, level, |_, h| {
                    touched = true;
                    tally.head_excess = tally.head_excess.max(h - level);
                });
                if touched {
                    tally.head += 1;
                    contact = Contact::Hit;
                }
            }
        }
        contact
    }
}

/// Le parcours redescend-il assez pour que la buse puisse toucher quelque
/// chose ? (Sinon, toute la matière est sous la pointe.)
fn descends(points: &[WayPoint], margin: f64) -> bool {
    let mut top = f64::NEG_INFINITY;
    for p in points {
        if p.z + margin < top {
            return true;
        }
        if p.kind.deposits() {
            top = top.max(p.z);
        }
    }
    false
}

/// Vérifie le parcours, marque les mouvements en cause (`Move::collision`) et
/// renvoie les messages.
pub fn check(
    path: &Toolpath,
    plan: &mut Plan,
    printer: &PrinterProfile,
    filament: &FilamentProfile,
) -> Vec<Issue> {
    let pts = &path.points;
    let Some(bounds) = path.bounds() else {
        return Vec::new();
    };
    if !descends(pts, MIN_MARGIN) {
        return Vec::new();
    }
    let n = &printer.nozzle;
    let shape = Shape {
        // Méplat de la pointe : à peu près le double de l'alésage.
        tip: n.diameter,
        cone: n
            .cone_angle
            .zip(n.cone_height)
            .map(|(a, h)| (a.to_radians().tan(), h)),
        head: n.clearance_radius.zip(n.clearance_height),
    };
    let mut map = HeightMap::new(
        [bounds.min[0], bounds.min[1]],
        [bounds.max[0], bounds.max[1]],
    );
    let mut tally = Tally::default();
    let layer = printer.layers.layer_height;
    // Prochain point à verser dans la carte, longueur de dépôt cumulée.
    let (mut pending, mut run) = (0_usize, vec![0.0_f64; pts.len()]);
    for i in 1..pts.len() {
        let (a, b) = (&pts[i - 1], &pts[i]);
        let d = (b.x - a.x).hypot(b.y - a.y).hypot(b.z - a.z);
        run[i] = run[i - 1] + if b.kind.deposits() { d } else { 0.0 };

        // Dépôt assez ancien pour que la buse l'ait quitté ; un déplacement
        // la sépare de tout ce qui précède.
        let keep = if b.kind.deposits() { 2.0 * b.w } else { 0.0 };
        while pending + 1 < i && (!b.kind.deposits() || run[i - 1] - run[pending + 1] > keep) {
            map.stamp_segment(&pts[pending], &pts[pending + 1]);
            pending += 1;
        }

        let (shoulder, margin, from, to) = match b.kind {
            MoveKind::Print | MoveKind::Bridge | MoveKind::Blob => (
                (b.w - b.h).max(0.0) / 2.0,
                b.h.max(MIN_MARGIN),
                [a.x, a.y, a.z],
                [b.x, b.y, b.z],
            ),
            MoveKind::Travel => {
                // Le déplacement avec rétraction passe au-dessus de ses deux
                // bouts, saut en Z compris (comme le G-code).
                let (_, _, hop) =
                    crate::path::Retraction::resolve(b.retraction, &filament.retraction);
                let z = if plan.moves[i].retract {
                    a.z.max(b.z) + hop
                } else {
                    a.z
                };
                let end = if plan.moves[i].retract { z } else { b.z };
                (0.0, layer.max(MIN_MARGIN), [a.x, a.y, z], [b.x, b.y, end])
            }
            // L'essuyage repasse sur le dernier tracé ; la pause ne bouge pas.
            MoveKind::Wipe | MoveKind::Pause => continue,
        };
        let steps = (d / STEP).ceil().max(1.0) as usize;
        let mut rubbed = false;
        for s in 1..=steps {
            let t = s as f64 / steps as f64;
            let at = [
                from[0] + (to[0] - from[0]) * t,
                from[1] + (to[1] - from[1]) * t,
                from[2] + (to[2] - from[2]) * t,
            ];
            match shape.test(&map, at, shoulder, margin, &mut tally) {
                Contact::Hit => {
                    plan.moves[i].collision = true;
                    break;
                }
                Contact::Rub => rubbed = true,
                Contact::None => {}
            }
        }
        if rubbed && !plan.moves[i].collision {
            tally.rub += 1;
        }
    }

    let mut issues = Vec::new();
    if tally.nozzle > 0 {
        issues.push(
            Issue::error("guard.nozzle_collision")
                .field("printer.nozzle")
                .count(plan.moves.iter().filter(|m| m.collision).count())
                .value(tally.nozzle_excess),
        );
    }
    if tally.head > 0 {
        issues.push(
            Issue::error("guard.head_collision")
                .field("printer.nozzle")
                .count(tally.head)
                .value(tally.head_excess),
        );
    }
    if tally.rub > 0 {
        issues.push(
            Issue::warning("guard.nozzle_rub")
                .field("printer.nozzle")
                .count(tally.rub)
                .value(tally.rub_excess),
        );
    }
    if tally.unknown_cone {
        issues.push(Issue::info("guard.cone_unknown").field("printer.nozzle"));
    }
    issues
}

#[cfg(test)]
mod tests {
    use crate::testing::{MK4S, PLA};
    use crate::Workbench;
    use serde_json::Map;

    /// Profil MK4S, avec un cône éventuel ; sans dégagement de la tête s'il
    /// n'est pas demandé (45 mm autour de la buse, 13 mm libres sinon).
    fn profile(cone: Option<(f64, f64)>, head: bool) -> String {
        let mut nozzle = String::from("[nozzle]\n");
        if let Some((a, h)) = cone {
            nozzle += &format!("cone_angle = {a}\ncone_height = {h}\n");
        }
        MK4S.replacen("[nozzle]\n", &nozzle, 1)
            .lines()
            .filter(|l| head || !l.starts_with("clearance_"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn bench(body: &str, cone: Option<(f64, f64)>, head: bool) -> Workbench {
        let mut w = Workbench::default();
        w.set_printer(&profile(cone, head)).unwrap();
        w.set_filament(PLA).unwrap();
        w.load_model(&format!(
            "[meta]\nname = \"t\"\ncategory = \"t\"\n---\nlet path = new_path();\n{body}\npath\n"
        ))
        .unwrap();
        w
    }

    /// Calcule un script d'essai : codes du garde-buse, mouvements surlignés.
    fn run(body: &str, cone: Option<(f64, f64)>, head: bool) -> (Vec<&'static str>, usize) {
        let ev = bench(body, cone, head).evaluate(&Map::new());
        let codes = ev
            .issues
            .iter()
            .filter(|i| i.code.starts_with("guard."))
            .map(|i| i.code)
            .collect();
        let n = ev
            .plan
            .as_ref()
            .map_or(0, |p| p.moves.iter().filter(|m| m.collision).count());
        (codes, n)
    }

    /// Mur droit de `layers` couches à x = 0, de 40 mm de long.
    fn wall(layers: usize) -> String {
        format!(
            "path.travel_to(0.0, -20.0, 0.2);\n\
             for k in 0..{layers} {{\n\
                 let z = 0.2 + k * 0.2;\n\
                 path.print_to(0.0, if k % 2 == 0 {{ 20.0 }} else {{ -20.0 }}, z, 0.45, 0.2);\n\
                 path.print_to(0.0, if k % 2 == 0 {{ 20.0 }} else {{ -20.0 }}, z + 0.2, 0.45, 0.2);\n\
             }}\n"
        )
    }

    /// Passe à côté du mur, à la hauteur `z`, à `x` de son axe.
    fn pass(x: f64, z: f64) -> String {
        format!(
            "path.travel_to({x}, -20.0, {z});\n\
             path.print_to({x}, 20.0, {z}, 0.45, 0.2);\n"
        )
    }

    #[test]
    fn a_rising_path_is_not_even_looked_at() {
        let (codes, n) = run(&wall(10), Some((45.0, 3.0)), true);
        assert!(codes.is_empty(), "{codes:?}");
        assert_eq!(n, 0);
    }

    #[test]
    fn a_nozzle_through_the_wall_is_a_collision() {
        // Sans cône ni tête : le bout suffit. Le mur monte à 2 mm.
        let body = format!("{}{}", wall(10), pass(0.1, 0.4));
        let (codes, n) = run(&body, None, false);
        assert!(codes.contains(&"guard.nozzle_collision"), "{codes:?}");
        assert!(n > 0);
    }

    #[test]
    fn the_cone_hits_a_wall_the_tip_clears() {
        // À 1,5 mm du mur, le bout passe ; un cône à 45° (permis : ≈ 1 mm au
        // mur) ne passe pas sous 2 mm de matière, un cône plat encore moins.
        let body = format!("{}{}", wall(15), pass(1.5, 0.2));
        let (without, _) = run(&body, None, false);
        assert_eq!(without, ["guard.cone_unknown"]);
        let (with, n) = run(&body, Some((45.0, 5.0)), false);
        assert!(with.contains(&"guard.nozzle_collision"), "{with:?}");
        assert!(n > 0);
        // Assez loin : le cône passe.
        let far = format!("{}{}", wall(15), pass(4.0, 0.2));
        let (codes, _) = run(&far, Some((45.0, 5.0)), false);
        assert!(codes.is_empty(), "{codes:?}");
    }

    #[test]
    fn the_head_hits_a_tall_part_within_its_radius() {
        // Mur de 20 mm, passe à 25 mm de lui : la tête (rayon 45, 13 mm libres)
        // le touche ; à 50 mm, non.
        let near = format!("{}{}", wall(100), pass(25.0, 0.2));
        let (codes, n) = run(&near, None, true);
        assert!(codes.contains(&"guard.head_collision"), "{codes:?}");
        assert!(n > 0);
        let far = format!("{}{}", wall(100), pass(50.0, 0.2));
        let (codes, _) = run(&far, None, true);
        assert!(codes.is_empty(), "{codes:?}");
    }

    #[test]
    fn a_shallow_touch_is_a_warning() {
        // Passe sur le mur, 0,4 mm sous son sommet : excès de 0,2 mm au-dessus
        // de la marge (un boudin), sous le seuil du heurt.
        let body = format!("{}{}", wall(4), pass(0.0, 0.6));
        let (codes, n) = run(&body, None, false);
        assert!(codes.contains(&"guard.nozzle_rub"), "{codes:?}");
        assert_eq!(n, 0);
    }

    #[test]
    fn a_travel_is_lifted_by_its_hop_but_not_without_one() {
        // Le déplacement traverse le mur : sous lui sans saut.
        let cross = "path.travel_to(-10.0, 0.0, 0.4);\n\
                     path.travel_to(10.0, 0.0, 0.4);\n\
                     path.print_to(10.0, 5.0, 0.4, 0.45, 0.2);\n";
        let (codes, _) = run(&format!("{}{cross}", wall(10)), None, false);
        assert!(codes.contains(&"guard.nozzle_collision"), "{codes:?}");
        // Au-dessus du mur : rien.
        let over = "path.travel_to(-10.0, 0.0, 5.0);\n\
                    path.travel_to(10.0, 0.0, 5.0);\n\
                    path.print_to(10.0, 5.0, 5.0, 0.45, 0.2);\n";
        let (codes, _) = run(&format!("{}{over}", wall(10)), None, false);
        assert!(codes.is_empty(), "{codes:?}");
    }

    #[test]
    fn a_collision_blocks_the_export() {
        let w = bench(&format!("{}{}", wall(10), pass(0.1, 0.4)), None, false);
        let ev = w.evaluate(&Map::new());
        assert!(w.gcode(&ev).is_err());
    }
}
