// Ruido Simplex 3D hecho a mano (Gustavson) + fBm + hash para patrones celulares.
use crate::math::{Vec3, vec3};

/// Tabla de permutación (duplicada a 512) barajada con un LCG fijo: resultado determinista.
const fn build_perm() -> [u8; 512] {
    let mut p = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        p[i] = i as u8;
        i += 1;
    }
    let mut s: u32 = 0x9E37_79B9;
    let mut i = 255;
    while i > 0 {
        s = s.wrapping_mul(1664525).wrapping_add(1013904223);
        let j = ((s >> 8) as usize) % (i + 1);
        let t = p[i];
        p[i] = p[j];
        p[j] = t;
        i -= 1;
    }
    let mut out = [0u8; 512];
    let mut i = 0;
    while i < 512 {
        out[i] = p[i & 255];
        i += 1;
    }
    out
}

const PERM: [u8; 512] = build_perm();

const GRAD: [[f32; 3]; 12] = [
    [1.0, 1.0, 0.0], [-1.0, 1.0, 0.0], [1.0, -1.0, 0.0], [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0], [-1.0, 0.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0], [0.0, -1.0, 1.0], [0.0, 1.0, -1.0], [0.0, -1.0, -1.0],
];

const F3: f32 = 1.0 / 3.0;
const G3: f32 = 1.0 / 6.0;

/// Simplex noise 3D, rango aproximado [-1, 1].
pub fn simplex3(p: Vec3) -> f32 {
    let s = (p.x + p.y + p.z) * F3;
    let (fi, fj, fk) = ((p.x + s).floor(), (p.y + s).floor(), (p.z + s).floor());
    let t = (fi + fj + fk) * G3;
    let (x0, y0, z0) = (p.x - (fi - t), p.y - (fj - t), p.z - (fk - t));

    // A qué símplex (tetraedro) pertenece el punto.
    let (i1, j1, k1, i2, j2, k2) = if x0 >= y0 {
        if y0 >= z0 { (1, 0, 0, 1, 1, 0) }
        else if x0 >= z0 { (1, 0, 0, 1, 0, 1) }
        else { (0, 0, 1, 1, 0, 1) }
    } else if y0 < z0 { (0, 0, 1, 0, 1, 1) }
    else if x0 < z0 { (0, 1, 0, 0, 1, 1) }
    else { (0, 1, 0, 1, 1, 0) };

    let corners = [
        (0, 0, 0, x0, y0, z0),
        (i1, j1, k1, x0 - i1 as f32 + G3, y0 - j1 as f32 + G3, z0 - k1 as f32 + G3),
        (i2, j2, k2, x0 - i2 as f32 + 2.0 * G3, y0 - j2 as f32 + 2.0 * G3, z0 - k2 as f32 + 2.0 * G3),
        (1, 1, 1, x0 - 1.0 + 3.0 * G3, y0 - 1.0 + 3.0 * G3, z0 - 1.0 + 3.0 * G3),
    ];
    let (ii, jj, kk) = (fi as i32 & 255, fj as i32 & 255, fk as i32 & 255);

    let mut n = 0.0;
    for (di, dj, dk, x, y, z) in corners {
        let t = 0.6 - x * x - y * y - z * z;
        if t > 0.0 {
            let h = PERM[(ii + di) as usize + PERM[(jj + dj) as usize + PERM[(kk + dk) as usize] as usize] as usize];
            let g = GRAD[h as usize % 12];
            let t2 = t * t;
            n += t2 * t2 * (g[0] * x + g[1] * y + g[2] * z);
        }
    }
    32.0 * n
}

/// fBm: suma de octavas de simplex. Normalizado a ~[-1, 1].
pub fn fbm(p: Vec3, octaves: u32, lacunarity: f32, gain: f32) -> f32 {
    let (mut sum, mut amp, mut norm) = (0.0, 1.0, 0.0);
    let mut q = p;
    for _ in 0..octaves {
        sum += amp * simplex3(q);
        norm += amp;
        amp *= gain;
        q = q * lacunarity + vec3(17.3, 9.1, 5.7); // desfase para romper alineación entre octavas
    }
    sum / norm
}

/// Atajo con lacunaridad 2 y ganancia 0.5.
pub fn fbm_std(p: Vec3, octaves: u32) -> f32 {
    fbm(p, octaves, 2.0, 0.5)
}

/// Hash entero 3D -> [0, 1).
pub fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}
