// Pelota de béisbol: cuero blanco y costura roja con puntadas en "V". La costura es la curva
// y = A·sin(2φ) (φ = longitud alrededor del eje Y), como la de una pelota de tenis; se mide la
// distancia con signo a la curva para colocar las puntadas a ambos lados.
use super::*;
use crate::noise::fbm_std;

const AMPLITUDE: f32 = 0.62;    // altura máxima de la costura (|y| ≤ A)
const STITCH_COUNT: f32 = 34.0; // puntadas por vuelta alrededor del eje
const STITCH_INNER: f32 = 0.018; // distancia de la costura donde empieza la puntada
const STITCH_OUTER: f32 = 0.11; // distancia donde termina
const STITCH_SLANT: f32 = 3.0;  // inclinación de la V
const STITCH_DUTY: f32 = 0.45;  // fracción del periodo que ocupa cada puntada
const BUMP_STRENGTH: f32 = 0.9;
const AMBIENT: f32 = 0.05;

const LEATHER: Vec3 = vec3(0.93, 0.91, 0.84);
const LEATHER_SHADE: Vec3 = vec3(0.78, 0.75, 0.68);
const RED: Vec3 = vec3(0.72, 0.05, 0.06);

/// (distancia con signo a la costura, longitud φ)
fn seam(p: Vec3) -> (f32, f32) {
    let phi = p.z.atan2(p.x);
    let r = (p.x * p.x + p.z * p.z).sqrt().max(0.05);
    let g = p.y - AMPLITUDE * (2.0 * phi).sin();
    let dg = 2.0 * AMPLITUDE * (2.0 * phi).cos();
    let grad = (r * r + dg * dg / (r * r)).sqrt();
    // Cerca de los polos la aproximación falla: se anula allí.
    let valid = 1.0 - smoothstep(AMPLITUDE + 0.1, AMPLITUDE + 0.25, p.y.abs());
    (g / grad + (1.0 - valid) * 10.0, phi)
}

/// Intensidad de las puntadas rojas en p.
fn stitches(p: Vec3) -> f32 {
    let (sd, phi) = seam(p);
    let side = sd.signum();
    let a = sd.abs();
    let band = smoothstep(STITCH_INNER, STITCH_INNER + 0.01, a) * (1.0 - smoothstep(STITCH_OUTER - 0.012, STITCH_OUTER, a));
    // Puntadas en V: la fase se desplaza con la distancia, en sentido opuesto a cada lado.
    let phase = (phi / std::f32::consts::TAU * STITCH_COUNT + side * a * STITCH_SLANT).rem_euclid(1.0);
    let tick = smoothstep(0.0, 0.08, phase) * (1.0 - smoothstep(STITCH_DUTY, STITCH_DUTY + 0.08, phase));
    band * tick
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = rotate_y(f.pos.normalize(), -u.time * BALL_SPIN);
    let (sd, _) = seam(p);

    let tone = 0.5 + 0.5 * fbm_std(p * 9.0 + vec3(1.0, 8.0, 2.0), 3);
    let mut albedo = LEATHER_SHADE.lerp(LEATHER, 0.45 + 0.55 * tone);
    // Sombra de la hendidura bajo la costura.
    albedo = albedo * (1.0 - 0.25 * (1.0 - smoothstep(0.0, 0.05, sd.abs())));
    albedo = albedo.lerp(RED, stitches(p));

    // Relieve: costura elevada + puntadas + veteado fino del cuero.
    let height = |q: Vec3| {
        let (d, _) = seam(q);
        0.012 * (1.0 - smoothstep(0.0, 0.03, d.abs())) + 0.008 * stitches(q) + 0.0004 * fbm_std(q * 35.0, 1)
    };
    let n_local = bump_normal(p, p, 0.006, BUMP_STRENGTH * 1.0, height);
    let delta = n_local - p;
    let n_s = (f.normal + rotate_y(delta, u.time * BALL_SPIN)).normalize();

    ball_light(albedo, n_s, f, u, AMBIENT, 35.0, 0.25)
}
