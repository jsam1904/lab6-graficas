// Fragment shaders procedurales (CPU). Un módulo por cuerpo celeste.
use crate::math::*;

pub mod baseball;
pub mod basketball;
pub mod gas_giant;
pub mod rocky;
pub mod soccer;
pub mod star;

/// Atributos interpolados que recibe el fragment shader.
/// La matriz de modelo es la identidad, así que `pos` (espacio de objeto) coincide con el mundo.
pub struct Fragment {
    pub pos: Vec3,      // posición en espacio de objeto
    pub normal: Vec3,   // normal unitaria
    #[allow(dead_code)]
    pub uv: (f32, f32), // UV del OBJ (los shaders usan la posición 3D para evitar costuras)
    pub view_dir: Vec3, // unitario, del fragmento hacia la cámara
}

pub struct Uniforms {
    pub time: f32,           // segundos de animación (avanza por frame)
    pub light_dir: Vec3,     // unitario, del fragmento hacia la luz
    pub cloud_coverage: f32, // 0..1, cobertura de nubes del planeta rocoso
}

#[derive(Clone, Copy, PartialEq)]
pub enum Body {
    Star,
    Rocky,
    GasGiant,
    Soccer,
    Basketball,
    Baseball,
}

pub fn shade(body: Body, f: &Fragment, u: &Uniforms) -> Vec3 {
    match body {
        Body::Star => star::shade(f, u),
        Body::Rocky => rocky::shade(f, u),
        Body::GasGiant => gas_giant::shade(f, u),
        Body::Soccer => soccer::shade(f, u),
        Body::Basketball => basketball::shade(f, u),
        Body::Baseball => baseball::shade(f, u),
    }
}

/// Bump mapping genérico: perturba la normal `n` con el gradiente de `height` en el plano tangente.
pub fn bump_normal(n: Vec3, p: Vec3, eps: f32, strength: f32, height: impl Fn(Vec3) -> f32) -> Vec3 {
    let up = if n.y.abs() > 0.95 { vec3(1.0, 0.0, 0.0) } else { vec3(0.0, 1.0, 0.0) };
    let t1 = n.cross(up).normalize();
    let t2 = n.cross(t1);
    let h0 = height(p);
    let g1 = (height(p + t1 * eps) - h0) / eps;
    let g2 = (height(p + t2 * eps) - h0) / eps;
    (n - (t1 * g1 + t2 * g2) * strength).normalize()
}

/// Iluminación de una pelota: ambiente + difusa (con terminador suave) + Blinn-Phong.
pub fn ball_light(albedo: Vec3, n_s: Vec3, f: &Fragment, u: &Uniforms, ambient: f32, spec_power: f32, spec_strength: f32) -> Vec3 {
    let l = u.light_dir;
    let lit = smoothstep(-0.1, 0.1, f.normal.dot(l));
    let diffuse = n_s.dot(l).max(0.0) * lit;
    let half = (l + f.view_dir).normalize();
    let spec = spec_strength * n_s.dot(half).max(0.0).powf(spec_power) * lit;
    albedo * (ambient + diffuse) + vec3(1.0, 0.98, 0.94) * spec
}

/// Giro de la pelota (rad/s). 0 = fija, igual que el terreno de los planetas; la cámara (flechas) las rota.
pub const BALL_SPIN: f32 = 0.0;
