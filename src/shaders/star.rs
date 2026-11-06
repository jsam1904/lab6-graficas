// Estrella (sol): emisiva, no usa la luz externa.
// Capas: granulación fBm con domain warping animado -> rampa de temperatura ->
// manchas solares (ruido umbralizado, deriva lenta) -> limb darkening + halo en el borde.
use super::{Fragment, Uniforms};
use crate::math::*;
use crate::noise::fbm_std;

// --- Turbulencia (granulación) ---
const TURB_FREQ: f32 = 2.6;       // más alto = granos más pequeños
const TURB_SPEED: f32 = 0.18;     // velocidad con la que hierve la superficie
const WARP_STRENGTH: f32 = 0.9;   // cuánto se deforma el ruido (más = más filamentos)
const TURB_OCTAVES: u32 = 4;

// --- Manchas solares ---
const SPOT_FREQ: f32 = 1.5;       // tamaño de las manchas (más bajo = más grandes)
const SPOT_UMBRA: f32 = 0.71;     // umbral del núcleo oscuro (más bajo = más/ más grandes manchas)
const SPOT_PENUMBRA: f32 = 0.62;  // umbral de la zona de transición
const SPOT_DRIFT: f32 = 0.05;     // rotación lenta de las manchas (rad/s)
const SPOT_EVOLVE: f32 = 0.08;    // qué tan rápido nacen/mueren las manchas
const SPOT_MAX_LAT: f32 = 0.8;    // |y| a partir del cual desaparecen (cerca de los polos)

// --- Borde ---
const LIMB_POWER: f32 = 0.6;      // oscurecimiento hacia el borde (más alto = más oscuro)
const RIM_GLOW: f32 = 0.35;       // intensidad del brillo naranja en el borde

// --- Colores (HDR, pueden pasar de 1) ---
const COL_DEEP: Vec3 = vec3(0.55, 0.06, 0.01);   // zona fría: rojo oscuro
const COL_ORANGE: Vec3 = vec3(1.0, 0.38, 0.04);  // naranja
const COL_YELLOW: Vec3 = vec3(1.15, 0.72, 0.14); // amarillo
const COL_WHITE: Vec3 = vec3(1.5, 1.3, 0.8);     // núcleo blanco-amarillo
const COL_SPOT: Vec3 = vec3(0.07, 0.008, 0.0);    // umbra de mancha

/// Rampa de temperatura t in [0,1] -> color.
fn temperature_ramp(t: f32) -> Vec3 {
    if t < 0.35 {
        COL_DEEP.lerp(COL_ORANGE, smoothstep(0.0, 0.35, t))
    } else if t < 0.7 {
        COL_ORANGE.lerp(COL_YELLOW, smoothstep(0.35, 0.7, t))
    } else {
        COL_YELLOW.lerp(COL_WHITE, smoothstep(0.7, 1.0, t))
    }
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.pos.normalize(); // ruido con posición 3D: sin costuras
    let t = u.time;
    let mu = clamp01(f.normal.dot(f.view_dir)); // 1 en el centro del disco, 0 en el borde

    // Granulación: fBm deformado por otro fBm, ambos desplazados en el tiempo.
    let q = p * TURB_FREQ;
    let warp = vec3(
        fbm_std(q + vec3(0.0, t * TURB_SPEED, 3.1), 3),
        fbm_std(q + vec3(t * TURB_SPEED * 0.7, 7.7, 0.0), 3),
        fbm_std(q + vec3(5.3, 1.9, -t * TURB_SPEED), 3),
    );
    let turb = fbm_std(q + warp * WARP_STRENGTH + vec3(0.0, 0.0, t * TURB_SPEED * 0.5), TURB_OCTAVES);
    let granule = 0.5 + 0.5 * turb;

    // Temperatura: el centro del disco es más caliente; la granulación añade zonas calientes/frías.
    let heat = clamp01(-0.1 + 0.8 * mu.powf(1.2) + 0.7 * (granule - 0.45));
    let mut col = temperature_ramp(heat);

    // Manchas solares: ruido umbralizado con deriva (rotación) y evolución en el tiempo.
    let ps = rotate_y(p, t * SPOT_DRIFT) * SPOT_FREQ;
    let dens = 0.5 + 0.5 * fbm_std(ps + vec3(11.0, 3.0, 7.0 + t * SPOT_EVOLVE), 3);
    let lat_fade = 1.0 - smoothstep(SPOT_MAX_LAT - 0.2, SPOT_MAX_LAT, p.y.abs());
    let dens = dens * lat_fade;
    let penumbra = smoothstep(SPOT_PENUMBRA, SPOT_PENUMBRA + 0.1, dens);
    let umbra = smoothstep(SPOT_UMBRA, SPOT_UMBRA + 0.06, dens);
    col = col.lerp(col.mul_v(vec3(0.45, 0.3, 0.2)), penumbra * 0.8);
    col = col.lerp(COL_SPOT, umbra);

    // Limb darkening: más oscuro y rojizo hacia el borde, más un halo cálido fino.
    let limb = 1.0 - LIMB_POWER * (1.0 - mu.powf(0.5));
    col = col * limb;
    col = col.mul_v(vec3(1.0, 0.75 + 0.25 * mu, 0.55 + 0.45 * mu));
    col + COL_ORANGE * (RIM_GLOW * (1.0 - mu).powf(4.0))
}
