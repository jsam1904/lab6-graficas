// Framebuffer + z-buffer + rasterización de triángulos con interpolación de atributos.
// Pipeline: (1) vértices -> pantalla, (2) rasterizado con z-buffer que guarda triángulo y
// coordenadas baricéntricas con corrección de perspectiva, (3) fragment shader por píxel
// (en paralelo por bandas de filas) que recibe posición, normal y uv interpolados.
use crate::math::{Vec3, vec3};
use crate::noise::hash3;
use crate::obj::Mesh;
use crate::shaders::{self, Body, Fragment, Uniforms};

/// Si es true la normal del fragmento es normalize(posición): esfera lisa aunque el mesh sea
/// de pocas caras (el .obj trae normales planas por cara). Con false se usa la del OBJ.
const SMOOTH_SPHERE_NORMALS: bool = true;
const NO_TRI: u32 = u32::MAX;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub color: Vec<u32>, // 0x00RRGGBB
    depth: Vec<f32>,
    tri: Vec<u32>,
    bary: Vec<[f32; 3]>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        Framebuffer {
            width,
            height,
            color: vec![0; n],
            depth: vec![f32::INFINITY; n],
            tri: vec![NO_TRI; n],
            bary: vec![[0.0; 3]; n],
        }
    }

    fn clear(&mut self) {
        self.depth.fill(f32::INFINITY);
        self.tri.fill(NO_TRI);
    }
}

pub struct Camera {
    pub pos: Vec3,
    right: Vec3,
    up: Vec3,
    fwd: Vec3,
    focal: f32, // en píxeles
}

impl Camera {
    /// Cámara orbital mirando al origen.
    pub fn orbit(yaw: f32, pitch: f32, dist: f32, fb_h: usize, fov_deg: f32) -> Self {
        let pos = vec3(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()) * dist;
        let fwd = (-pos).normalize();
        let right = fwd.cross(vec3(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(fwd);
        let focal = fb_h as f32 * 0.5 / (fov_deg.to_radians() * 0.5).tan();
        Camera { pos, right, up, fwd, focal }
    }

    /// Devuelve (x, y, profundidad) en píxeles.
    fn project(&self, p: Vec3, w: usize, h: usize) -> (f32, f32, f32) {
        let r = p - self.pos;
        let z = r.dot(self.fwd);
        (
            w as f32 * 0.5 + r.dot(self.right) / z * self.focal,
            h as f32 * 0.5 - r.dot(self.up) / z * self.focal,
            z,
        )
    }
}

fn edge(a: (f32, f32), b: (f32, f32), p: (f32, f32)) -> f32 {
    (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
}

fn rasterize(fb: &mut Framebuffer, mesh: &Mesh, cam: &Camera) {
    let (w, h) = (fb.width, fb.height);
    let proj: Vec<_> = mesh.verts.iter().map(|&v| cam.project(v, w, h)).collect();

    for (ti, t) in mesh.tris.iter().enumerate() {
        let wp = [mesh.verts[t[0].v], mesh.verts[t[1].v], mesh.verts[t[2].v]];
        // Culling de caras traseras con la normal geométrica.
        if (wp[1] - wp[0]).cross(wp[2] - wp[0]).dot(cam.pos - wp[0]) <= 0.0 {
            continue;
        }
        let s = [proj[t[0].v], proj[t[1].v], proj[t[2].v]];
        let (a, b, c) = ((s[0].0, s[0].1), (s[1].0, s[1].1), (s[2].0, s[2].1));
        let area = edge(a, b, c);
        if area.abs() < 1e-8 {
            continue;
        }
        let x0 = a.0.min(b.0).min(c.0).floor().max(0.0) as usize;
        let x1 = (a.0.max(b.0).max(c.0).ceil().max(0.0) as usize).min(w - 1);
        let y0 = a.1.min(b.1).min(c.1).floor().max(0.0) as usize;
        let y1 = (a.1.max(b.1).max(c.1).ceil().max(0.0) as usize).min(h - 1);
        let inv_z = [1.0 / s[0].2, 1.0 / s[1].2, 1.0 / s[2].2];

        for y in y0..=y1 {
            for x in x0..=x1 {
                let p = (x as f32 + 0.5, y as f32 + 0.5);
                let b0 = edge(b, c, p) / area;
                let b1 = edge(c, a, p) / area;
                let b2 = edge(a, b, p) / area;
                if b0 < 0.0 || b1 < 0.0 || b2 < 0.0 {
                    continue;
                }
                let iz = b0 * inv_z[0] + b1 * inv_z[1] + b2 * inv_z[2];
                let depth = 1.0 / iz;
                let i = y * w + x;
                if depth < fb.depth[i] {
                    fb.depth[i] = depth;
                    fb.tri[i] = ti as u32;
                    // Baricéntricas con corrección de perspectiva.
                    fb.bary[i] = [b0 * inv_z[0] * depth, b1 * inv_z[1] * depth, b2 * inv_z[2] * depth];
                }
            }
        }
    }
}

/// Fondo de estrellas fijo, derivado del hash de la posición del píxel.
fn background(x: usize, y: usize) -> u32 {
    let hsh = hash3(x as i32, y as i32, 7);
    if hsh > 0.9975 {
        let v = (90.0 + 165.0 * (hsh - 0.9975) / 0.0025) as u32;
        (v << 16) | (v << 8) | v
    } else {
        0x03040a
    }
}

fn to_rgb(c: Vec3) -> u32 {
    // Tonemapping suave (Reinhard ligero) y gamma 2.2 para que los HDR de la estrella no quemen.
    let f = |v: f32| {
        let v = v.max(0.0);
        let v = v / (1.0 + 0.15 * v);
        (v.min(1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u32
    };
    (f(c.x) << 16) | (f(c.y) << 8) | f(c.z)
}

pub fn render(fb: &mut Framebuffer, mesh: &Mesh, cam: &Camera, body: Body, uni: &Uniforms) {
    fb.clear();
    rasterize(fb, mesh, cam);

    let (w, h) = (fb.width, fb.height);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let rows = h.div_ceil(threads);
    let (tri, bary) = (&fb.tri, &fb.bary);

    std::thread::scope(|sc| {
        for (band, chunk) in fb.color.chunks_mut(rows * w).enumerate() {
            sc.spawn(move || {
                for (k, px) in chunk.iter_mut().enumerate() {
                    let i = band * rows * w + k;
                    let (x, y) = (i % w, i / w);
                    if tri[i] == NO_TRI {
                        *px = background(x, y);
                        continue;
                    }
                    let t = &mesh.tris[tri[i] as usize];
                    let b = bary[i];
                    let pos = mesh.verts[t[0].v] * b[0] + mesh.verts[t[1].v] * b[1] + mesh.verts[t[2].v] * b[2];
                    let normal = if SMOOTH_SPHERE_NORMALS {
                        pos.normalize()
                    } else {
                        (mesh.normals[t[0].vn] * b[0] + mesh.normals[t[1].vn] * b[1] + mesh.normals[t[2].vn] * b[2]).normalize()
                    };
                    let uv = |c: usize| mesh.uvs.get(t[c].vt).copied().unwrap_or((0.0, 0.0));
                    let (u0, u1, u2) = (uv(0), uv(1), uv(2));
                    let frag = Fragment {
                        pos,
                        normal,
                        uv: (u0.0 * b[0] + u1.0 * b[1] + u2.0 * b[2], u0.1 * b[0] + u1.1 * b[1] + u2.1 * b[2]),
                        view_dir: (cam.pos - pos).normalize(),
                    };
                    *px = to_rgb(shaders::shade(body, &frag, uni));
                }
            });
        }
    });
}
