// Planeta rocoso: océanos, costas, continentes, desierto/montañas y hielo polar (fBm + umbrales),
// bump mapping procedural, océano especular (Blinn-Phong) vs. continentes mate,
// luces de ciudades en el lado nocturno, nubes con velocidad propia y sombras de nubes.
use super::{Fragment, Uniforms};
use crate::math::*;
use crate::noise::{fbm_std, hash3};

// --- Terreno ---
const TERRAIN_FREQ: f32 = 1.7;    // tamaño de continentes (más alto = más islas pequeñas)
const TERRAIN_OCTAVES: u32 = 5;
const SEA_LEVEL: f32 = 0.04;      // más alto = más océano
const BEACH_WIDTH: f32 = 0.035;   // ancho de la franja de costa/arena
const MOUNTAIN_START: f32 = 0.28; // altura sobre el mar donde empieza la roca
const SNOW_START: f32 = 0.55;     // altura de las cumbres nevadas
const ICE_LATITUDE: f32 = 0.74;   // |y| donde empieza el hielo polar (más bajo = casquetes más grandes)
const MOISTURE_FREQ: f32 = 2.4;   // tamaño de las zonas verdes/desierto

// --- Bump mapping ---
const BUMP_STRENGTH: f32 = 0.5;   // inclinación de las normales (0 = terreno liso)
const BUMP_DETAIL_FREQ: f32 = 14.0; // frecuencia del relieve fino superpuesto
const BUMP_DETAIL_AMP: f32 = 0.012;  // amplitud del relieve fino
const BUMP_EPS: f32 = 0.012;      // paso de las diferencias finitas

// --- Iluminación ---
const AMBIENT: f32 = 0.025;
const SPEC_POWER: f32 = 90.0;     // más alto = reflejo del sol más pequeño y nítido (océano)
const SPEC_STRENGTH: f32 = 0.9;
const SUN_COLOR: Vec3 = vec3(1.0, 0.96, 0.88);

// --- Nubes ---
const CLOUD_FREQ: f32 = 2.9;      // tamaño de los bancos de nubes
const CLOUD_SPEED: f32 = 0.06;    // rad/s de giro de las nubes (el terreno está fijo)
const CLOUD_SOFTNESS: f32 = 0.16; // bordes difusos de las nubes
const CLOUD_OPACITY: f32 = 0.93;
// La cobertura (0..1) viene de Uniforms::cloud_coverage (teclas Z / X).
const SHADOW_OFFSET: f32 = 0.06;  // desplazamiento de la sombra hacia la luz (más = sombra más separada)
const SHADOW_DARKNESS: f32 = 0.55;

// --- Día y noche ---
const NIGHT_THRESHOLD: f32 = 0.06; // n·L por debajo del cual se ven las luces
const CITY_FREQ: f32 = 32.0;       // densidad de ciudades (tamaño de la cuadrícula)
const CITY_COLOR: Vec3 = vec3(1.0, 0.72, 0.3);

// --- Paleta ---
const OCEAN_DEEP: Vec3 = vec3(0.01, 0.05, 0.22);
const OCEAN_SHALLOW: Vec3 = vec3(0.04, 0.34, 0.52);
const SAND: Vec3 = vec3(0.82, 0.74, 0.47);
const GRASS: Vec3 = vec3(0.14, 0.40, 0.12);
const DESERT: Vec3 = vec3(0.72, 0.52, 0.28);
const ROCK: Vec3 = vec3(0.38, 0.33, 0.30);
const SNOW: Vec3 = vec3(0.95, 0.96, 1.0);
const ICE: Vec3 = vec3(0.86, 0.93, 1.0);

const TERRAIN_SEED: Vec3 = vec3(3.7, 1.3, 8.9);

/// Altura del terreno en [-1, 1] aprox. (fBm 3D sobre la esfera unitaria).
fn terrain(p: Vec3) -> f32 {
    fbm_std(p * TERRAIN_FREQ + TERRAIN_SEED, TERRAIN_OCTAVES)
}

/// Campo de altura para el bump: continentes + detalle fino; el océano es plano.
fn bump_height(p: Vec3) -> f32 {
    let h = terrain(p) - SEA_LEVEL;
    if h <= 0.0 {
        return 0.0;
    }
    h + BUMP_DETAIL_AMP * fbm_std(p * BUMP_DETAIL_FREQ, 2) * smoothstep(0.0, 0.1, h)
}

/// Densidad de nubes [0,1] con la coordenada rotada en Y (velocidad propia).
fn cloud_mask(p: Vec3, time: f32, coverage: f32) -> f32 {
    let q = rotate_y(p, time * CLOUD_SPEED) * CLOUD_FREQ + vec3(31.0, 7.0, 13.0);
    let d = 0.5 + 0.5 * fbm_std(q, 4);
    let thr = mix(0.78, 0.32, coverage); // más cobertura = umbral más bajo
    smoothstep(thr, thr + CLOUD_SOFTNESS, d)
}

/// Luces de ciudades: puntos en una cuadrícula 3D, agrupados por una máscara de población.
fn city_lights(p: Vec3) -> f32 {
    let q = p * CITY_FREQ;
    let (ix, iy, iz) = (q.x.floor(), q.y.floor(), q.z.floor());
    let cell = vec3(q.x - ix, q.y - iy, q.z - iz);
    let (cx, cy, cz) = (ix as i32, iy as i32, iz as i32);
    let exists = hash3(cx, cy, cz);
    let center = vec3(0.3 + 0.4 * hash3(cx + 1, cy, cz), 0.3 + 0.4 * hash3(cx, cy + 1, cz), 0.3 + 0.4 * hash3(cx, cy, cz + 1));
    let dot = smoothstep(0.32, 0.05, (cell - center).length());
    let population = smoothstep(-0.1, 0.25, fbm_std(p * 3.2 + vec3(9.0, 2.0, 4.0), 3));
    dot * smoothstep(0.35, 0.6, exists) * population * (0.5 + exists)
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = f.pos.normalize();
    let n = f.normal;
    let l = u.light_dir;

    // --- Terreno y bioma ---
    let h = terrain(p);
    let elev = h - SEA_LEVEL;
    let land_w = smoothstep(0.0, 0.012, elev); // 0 océano, 1 tierra

    // Océano: más profundo = más oscuro.
    let depth = clamp01(-elev / 0.55);
    let ocean = OCEAN_SHALLOW.lerp(OCEAN_DEEP, smoothstep(0.0, 1.0, depth).powf(0.7));

    // Tierra: verde/desierto por humedad, roca por altura, nieve en cumbres.
    let moisture = 0.5 + 0.5 * fbm_std(p * MOISTURE_FREQ + vec3(21.0, 5.0, 2.0), 3);
    let lowland = DESERT.lerp(GRASS, smoothstep(0.38, 0.58, moisture));
    let mut land = SAND.lerp(lowland, smoothstep(0.0, BEACH_WIDTH, elev));
    land = land.lerp(ROCK, smoothstep(MOUNTAIN_START, MOUNTAIN_START + 0.12, elev));
    land = land.lerp(SNOW, smoothstep(SNOW_START, SNOW_START + 0.08, elev));
    let mut albedo = ocean.lerp(land, land_w);

    // Hielo polar por latitud, con borde irregular por ruido.
    let ice_n = fbm_std(p * 4.0 + vec3(2.0, 8.0, 5.0), 3);
    let ice_w = smoothstep(ICE_LATITUDE, ICE_LATITUDE + 0.07, p.y.abs() + 0.08 * ice_n);
    albedo = albedo.lerp(ICE, ice_w);

    // --- Bump mapping: gradiente del campo de altura en el plano tangente ---
    let up = if n.y.abs() > 0.95 { vec3(1.0, 0.0, 0.0) } else { vec3(0.0, 1.0, 0.0) };
    let t1 = n.cross(up).normalize();
    let t2 = n.cross(t1);
    let h0 = bump_height(p);
    let g1 = (bump_height(p + t1 * BUMP_EPS) - h0) / BUMP_EPS;
    let g2 = (bump_height(p + t2 * BUMP_EPS) - h0) / BUMP_EPS;
    let n_bump = (n - (t1 * g1 + t2 * g2) * BUMP_STRENGTH).normalize();
    // El océano conserva la normal lisa; tierra y hielo usan la perturbada.
    let n_s = n.lerp(n_bump, clamp01(land_w + ice_w)).normalize();

    // --- Iluminación ---
    let diffuse = n_s.dot(l).max(0.0) * smoothstep(-0.1, 0.1, n.dot(l)); // sin luz tras el terminador
    let facing = n.dot(l); // luz sobre la esfera lisa: define día/noche
    let half = (l + f.view_dir).normalize();
    // Blinn-Phong solo en océano (continentes y hielo mate).
    let ocean_w = (1.0 - land_w) * (1.0 - ice_w);
    let spec = ocean_w * SPEC_STRENGTH * n.dot(half).max(0.0).powf(SPEC_POWER) * smoothstep(0.0, 0.1, facing);

    let clouds = cloud_mask(p, u.time, u.cloud_coverage);

    // Sombras de nubes: la máscara se muestrea desplazada hacia la luz.
    let l_tan = l - n * n.dot(l);
    let shadow_clouds = cloud_mask((p + l_tan * SHADOW_OFFSET).normalize(), u.time, u.cloud_coverage);
    let shadow = 1.0 - SHADOW_DARKNESS * shadow_clouds * smoothstep(0.0, 0.2, facing);

    let mut col = albedo * (AMBIENT + diffuse * shadow) + SUN_COLOR * (spec * shadow);

    // --- Noche: ciudades solo en tierra firme sin hielo, tapadas por las nubes ---
    let night = smoothstep(NIGHT_THRESHOLD, NIGHT_THRESHOLD - 0.1, facing);
    let habitable = land_w * (1.0 - ice_w) * smoothstep(0.30, 0.15, elev) * smoothstep(0.01, 0.04, elev);
    col = col + CITY_COLOR * (city_lights(p) * night * habitable * (1.0 - clouds) * 2.2);

    // --- Nubes: se iluminan con la normal lisa; también tienen lado nocturno ---
    let cloud_light = AMBIENT * 1.5 + n.dot(l).max(0.0);
    let cloud_col = vec3(1.0, 1.0, 1.0) * cloud_light;
    col.lerp(cloud_col, clouds * CLOUD_OPACITY)
}
