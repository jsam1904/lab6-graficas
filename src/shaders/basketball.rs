// Balón de baloncesto: cuero naranja granulado (bump con ruido de alta frecuencia) y costuras negras
// hundidas: un ecuador, un meridiano y dos arcos laterales (círculos menores).
use super::*;
use crate::noise::fbm_std;

const LINE_WIDTH: f32 = 0.022;   // semiancho de las líneas negras
const LINE_SOFT: f32 = 0.008;
const SIDE_ARC: f32 = 0.62;      // |x| donde pasan los arcos laterales (más alto = más hacia el borde)
const PEBBLE_FREQ: f32 = 45.0;   // densidad del granulado
const PEBBLE_AMP: f32 = 0.0012;
const BUMP_STRENGTH: f32 = 1.2;
const AMBIENT: f32 = 0.05;

const ORANGE: Vec3 = vec3(0.86, 0.36, 0.07);
const ORANGE_DARK: Vec3 = vec3(0.62, 0.2, 0.03);
const LINE: Vec3 = vec3(0.02, 0.015, 0.012);

/// Distancia aproximada (sobre la esfera) a la línea más cercana.
fn line_dist(p: Vec3) -> f32 {
    let equator = p.y.abs();
    let meridian = p.x.abs();
    // Círculos menores en los planos x = ±SIDE_ARC; el radio es sqrt(1-c²), se escala la distancia.
    let side = ((p.x.abs() - SIDE_ARC).abs()) * 1.0;
    // Los arcos laterales solo existen lejos del eje x del meridiano y del ecuador cortan: se dejan completos.
    equator.min(meridian).min(side)
}

pub fn shade(f: &Fragment, u: &Uniforms) -> Vec3 {
    let p = rotate_y(f.pos.normalize(), -u.time * BALL_SPIN);
    let d = line_dist(p);
    let line = 1.0 - smoothstep(LINE_WIDTH, LINE_WIDTH + LINE_SOFT, d);

    // Variación suave de tono + granulado.
    let tone = 0.5 + 0.5 * fbm_std(p * 6.0 + vec3(3.0, 1.0, 7.0), 2);
    let leather = ORANGE_DARK.lerp(ORANGE, 0.55 + 0.45 * tone);
    let albedo = leather.lerp(LINE, line);

    let height = |q: Vec3| {
        PEBBLE_AMP * fbm_std(q * PEBBLE_FREQ, 1) - 0.012 * (1.0 - smoothstep(0.0, LINE_WIDTH * 2.0, line_dist(q)))
    };
    let n_local = bump_normal(p, p, 0.008, BUMP_STRENGTH * 1.5, height);
    let delta = n_local - p;
    let n_s = (f.normal + rotate_y(delta, u.time * BALL_SPIN)).normalize();

    ball_light(albedo, n_s, f, u, AMBIENT, 25.0, 0.3)
}
