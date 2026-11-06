// Balón de fútbol: icosaedro truncado. 12 pentágonos negros (vértices del icosaedro) y 20 hexágonos
// blancos (caras). Cada punto elige la "celda" con mayor puntuación = dot(p, centro) + sesgo;
// la diferencia entre las dos mejores puntuaciones da la distancia a la costura.
use super::*;
use crate::noise::fbm_std;

const HEX_BIAS: f32 = 0.0235;    // ajusta el tamaño relativo hexágono/pentágono (aristas iguales)
const SEAM_WIDTH: f32 = 0.012;   // ancho de las costuras (en diferencia de puntuación)
const SEAM_SOFT: f32 = 0.006;
const BUMP_STRENGTH: f32 = 0.5;
const AMBIENT: f32 = 0.05;

const WHITE: Vec3 = vec3(0.95, 0.95, 0.93);
const BLACK: Vec3 = vec3(0.025, 0.025, 0.03);
const SEAM: Vec3 = vec3(0.30, 0.30, 0.30);

const PHI: f32 = 1.618_034;

/// Centros de las 32 celdas: 12 pentágonos + 20 hexágonos (flag = es pentágono).
fn cells() -> [(Vec3, bool); 32] {
    let mut out = [(vec3(0.0, 0.0, 0.0), false); 32];
    let mut i = 0;
    // Vértices del icosaedro: permutaciones cíclicas de (0, ±1, ±φ).
    for &a in &[-1.0f32, 1.0] {
        for &b in &[-PHI, PHI] {
            for v in [vec3(0.0, a, b), vec3(a, b, 0.0), vec3(b, 0.0, a)] {
                out[i] = (v.normalize(), true);
                i += 1;
            }
        }
    }
    // Centros de cara (dodecaedro): (±1,±1,±1) y permutaciones cíclicas de (0, ±1/φ, ±φ).
    let inv = 1.0 / PHI;
    for &x in &[-1.0f32, 1.0] {
        for &y in &[-1.0f32, 1.0] {
            for &z in &[-1.0f32, 1.0] {
                out[i] = (vec3(x, y, z).normalize(), false);
                i += 1;
            }
        }
    }
    for &a in &[-inv, inv] {
        for &b in &[-PHI, PHI] {
            for v in [vec3(0.0, a, b), vec3(a, b, 0.0), vec3(b, 0.0, a)] {
                out[i] = (v.normalize(), false);
                i += 1;
            }
        }
    }
    out
}

/// (es_pentágono, distancia de puntuación a la costura)
fn panel(p: Vec3) -> (bool, f32) {
    let (mut best, mut second) = (f32::MIN, f32::MIN);
    let mut pent = false;
    for (c, is_pent) in cells() {
        let s = p.dot(c) + if is_pent { 0.0 } else { HEX_BIAS };
        if s > best {
            second = best;
            best = s;
            pent = is_pent;
        } else if s > second {
            second = s;
        }
    }
    (pent, best - second)
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = rotate_y(f.pos.normalize(), -u.time * BALL_SPIN);
    let (pent, gap) = panel(p);
    let seam = 1.0 - smoothstep(SEAM_WIDTH, SEAM_WIDTH + SEAM_SOFT, gap);

    let panel_col = if pent { BLACK } else { WHITE };
    let albedo = panel_col.lerp(SEAM, seam);

    // Cuero: textura fina + costuras hundidas.
    let height = |q: Vec3| {
        let (_, g) = panel(q);
        -0.02 * (1.0 - smoothstep(0.0, SEAM_WIDTH * 2.5, g)) + 0.0006 * fbm_std(q * 40.0, 1)
    };
    let n_local = bump_normal(p, p, 0.01, BUMP_STRENGTH * 0.8, height);
    // Devolver la normal perturbada al espacio de mundo deshaciendo el giro.
    let delta = n_local - p;
    let n_s = (f.normal + rotate_y(delta, u.time * BALL_SPIN)).normalize();

    ball_light(albedo, n_s, f, u, AMBIENT, 60.0, 0.55)
}
