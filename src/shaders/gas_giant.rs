// Gigante gaseoso: bandas por latitud con domain warping animado (transiciones "líquidas")
// y una tormenta tipo Gran Mancha Roja que curva las bandas con una torsión en espiral.
use super::{Fragment, Uniforms};
use crate::math::*;
use crate::noise::fbm_std;

// --- Bandas ---
const BAND_FREQ: f32 = 5.5;        // número de bandas (más alto = más bandas finas)
const WARP_FREQ: f32 = 2.2;        // tamaño de las ondulaciones que deforman las bandas
const WARP_STRENGTH: f32 = 0.16;   // cuánto se deforma la latitud (más = bandas más turbulentas)
const WARP_SPEED: f32 = 0.025;     // velocidad del warp (lento = fluido)
const STREAK_FREQ: f32 = 9.0;      // finura de las vetas dentro de cada banda
const STREAK_AMOUNT: f32 = 0.22;   // contraste de las vetas

// --- Tormenta ---
const STORM_CENTER_LAT: f32 = -0.30; // latitud (seno) del centro: -1 sur ... 1 norte
const STORM_CENTER_LON: f32 = 0.45;  // longitud del centro (rad)
const STORM_RADIUS: f32 = 0.38;      // radio angular de la tormenta (rad)
const STORM_TWIST: f32 = 5.5;        // fuerza de la espiral (rad de giro en el centro)
const STORM_SPIN: f32 = 0.05;        // rotación lenta de la espiral en el tiempo (rad/s)

// --- Iluminación ---
const AMBIENT: f32 = 0.05;

// --- Paleta de bandas ---
const CREAM: Vec3 = vec3(0.92, 0.82, 0.62);
const TAN: Vec3 = vec3(0.72, 0.47, 0.26);
const BROWN: Vec3 = vec3(0.36, 0.19, 0.10);
const STORM_RED: Vec3 = vec3(0.62, 0.11, 0.05);
const STORM_PALE: Vec3 = vec3(0.88, 0.40, 0.22);

fn band_palette(v: f32) -> Vec3 {
    if v < 0.5 {
        BROWN.lerp(TAN, smoothstep(0.0, 0.5, v))
    } else {
        TAN.lerp(CREAM, smoothstep(0.5, 1.0, v))
    }
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.pos.normalize();
    let t = u.time;

    // Centro y base local de la tormenta sobre la esfera.
    let cl = STORM_CENTER_LAT;
    let center = vec3(
        (1.0 - cl * cl).sqrt() * STORM_CENTER_LON.sin(),
        cl,
        (1.0 - cl * cl).sqrt() * STORM_CENTER_LON.cos(),
    );
    let r = clamp01(p.dot(center)).acos(); // distancia angular al centro

    // Distorsión en espiral: se gira el punto alrededor del centro; el ángulo cae con la distancia,
    // así las bandas (que dependen de la latitud) se enroscan alrededor de la tormenta.
    let falloff = (-(r / STORM_RADIUS).powi(2)).exp();
    let q = rotate_axis(p, center, STORM_TWIST * falloff + t * STORM_SPIN * falloff);

    // Domain warping: se perturba la latitud con fBm animado lentamente antes de calcular bandas.
    let wn = fbm_std(q * WARP_FREQ + vec3(0.0, t * WARP_SPEED, t * WARP_SPEED * 0.6), 4);
    let y = q.y + WARP_STRENGTH * wn;

    // Bandas: dos armónicos de seno para que no sean regulares.
    let band = 0.5 + 0.5 * (y * BAND_FREQ * std::f32::consts::PI * 0.5).sin();
    let band = clamp01(band * 0.75 + 0.25 * (0.5 + 0.5 * (y * BAND_FREQ * 2.3 + 1.7).sin()));

    // Vetas alargadas en longitud (se estira el ruido en Y).
    let streak = fbm_std(vec3(q.x * STREAK_FREQ * 0.5, y * STREAK_FREQ * 2.0, q.z * STREAK_FREQ * 0.5) + vec3(4.0, t * WARP_SPEED, 2.0), 3);
    let mut col = band_palette(clamp01(band + STREAK_AMOUNT * streak));

    // Interior de la tormenta: color propio con remolinos que siguen la espiral.
    let edge_n = 0.05 * fbm_std(p * 9.0 + vec3(1.0, 5.0, 3.0), 3);
    let storm_w = smoothstep(STORM_RADIUS * 0.85, STORM_RADIUS * 0.5, r + edge_n);
    let swirl = 0.5 + 0.5 * fbm_std(q * 6.0 + vec3(8.0, 2.0, 1.0), 3);
    let storm_col = STORM_RED.lerp(STORM_PALE, swirl * 0.8 + 0.2 * band);
    col = col.lerp(storm_col, storm_w * 0.95);
    // Anillo claro alrededor del borde de la tormenta.
    col = col.lerp(CREAM, 0.35 * smoothstep(STORM_RADIUS * 0.15, 0.0, (r - STORM_RADIUS * 0.95).abs()));

    let diffuse = f.normal.dot(u.light_dir).max(0.0);
    col * (AMBIENT + diffuse)
}
