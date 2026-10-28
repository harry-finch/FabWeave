//! Bruit de gradient 3D (API-17) : textures aléatoires mais continues
//! (reliefs, vermicelles, peau granuleuse). Même principe que le bruit de
//! Perlin : un gradient pseudo-aléatoire à chaque nœud d'une grille de pas
//! 1, interpolé par une courbe de degré 5 (dérivées première et seconde
//! continues). Les gradients viennent d'un hachage entier des coordonnées du
//! nœud, sans table : pur, déterministe, identique en natif et en
//! WebAssembly (ni trigonométrie, ni aléa du système).
//!
//! Valeur dans [-1, 1] (en pratique ±0,9 au plus), nulle sur chaque nœud,
//! moyenne nulle ; motifs d'une taille d'environ 1. Mettre les coordonnées
//! à l'échelle pour changer la taille, les décaler pour changer de tirage.

/// Douze gradients : milieux des arêtes d'un cube.
const GRADIENTS: [[f64; 3]; 12] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
];

/// Hachage entier d'un nœud (mélange multiplicatif, bits bien répartis).
fn hash(x: i64, y: i64, z: i64) -> usize {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (z as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 31;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 29;
    (h % 12) as usize
}

fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Bruit au point (x, y, z) ; 0 pour une coordonnée non finie.
pub fn noise3(x: f64, y: f64, z: f64) -> f64 {
    if !(x.is_finite() && y.is_finite() && z.is_finite()) {
        return 0.0;
    }
    let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
    let (ix, iy, iz) = (fx as i64, fy as i64, fz as i64);
    let (dx, dy, dz) = (x - fx, y - fy, z - fz);
    let corner = |i: i64, j: i64, k: i64| {
        let g = GRADIENTS[hash(ix + i, iy + j, iz + k)];
        g[0] * (dx - i as f64) + g[1] * (dy - j as f64) + g[2] * (dz - k as f64)
    };
    let (u, v, w) = (fade(dx), fade(dy), fade(dz));
    let x00 = lerp(corner(0, 0, 0), corner(1, 0, 0), u);
    let x10 = lerp(corner(0, 1, 0), corner(1, 1, 0), u);
    let x01 = lerp(corner(0, 0, 1), corner(1, 0, 1), u);
    let x11 = lerp(corner(0, 1, 1), corner(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_on_lattice_nodes_and_bounded() {
        assert_eq!(noise3(3.0, -2.0, 7.0), 0.0);
        let mut lo: f64 = 0.0;
        let mut hi: f64 = 0.0;
        let mut sum = 0.0;
        let n = 40_000;
        for i in 0..n {
            let t = i as f64 * 0.0137;
            let v = noise3(t * 1.3, t * 0.7 + 5.0, t * 0.31 - 2.0);
            lo = lo.min(v);
            hi = hi.max(v);
            sum += v;
        }
        assert!(lo >= -1.0 && hi <= 1.0, "{lo} {hi}");
        // Des deux côtés de zéro, et centré.
        assert!(lo < -0.4 && hi > 0.4, "{lo} {hi}");
        assert!((sum / n as f64).abs() < 0.05, "{}", sum / n as f64);
    }

    #[test]
    fn continuous_and_deterministic() {
        let a = noise3(1.234, 5.678, -9.1);
        assert_eq!(a, noise3(1.234, 5.678, -9.1));
        let b = noise3(1.234 + 1e-6, 5.678, -9.1);
        assert!((a - b).abs() < 1e-5);
        // Une valeur de référence : tout changement du calcul se voit.
        assert_eq!(
            format!("{:.12}", noise3(-3.7, 12.1, 0.9)),
            "-0.044254215642"
        );
        assert_eq!(noise3(f64::NAN, 0.0, 0.0), 0.0);
    }
}
