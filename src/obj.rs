// Cargador OBJ mínimo: v, vt, vn y caras v/vt/vn (polígonos se triangulan en abanico).
use crate::math::{Vec3, vec3};
use std::fs;

#[derive(Clone, Copy)]
pub struct Corner {
    pub v: usize,
    pub vt: usize,
    pub vn: usize,
}

pub struct Mesh {
    pub verts: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<(f32, f32)>,
    pub tris: Vec<[Corner; 3]>,
}

pub fn load(path: &str) -> Result<Mesh, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut m = Mesh { verts: vec![], normals: vec![], uvs: vec![], tris: vec![] };
    let num = |s: Option<&str>| s.and_then(|s| s.parse::<f32>().ok()).unwrap_or(0.0);
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("v") => m.verts.push(vec3(num(it.next()), num(it.next()), num(it.next()))),
            Some("vn") => m.normals.push(vec3(num(it.next()), num(it.next()), num(it.next())).normalize()),
            Some("vt") => m.uvs.push((num(it.next()), num(it.next()))),
            Some("f") => {
                let corners: Vec<Corner> = it
                    .map(|tok| {
                        let mut p = tok.split('/').map(|s| s.parse::<usize>().ok());
                        let v = p.next().flatten().unwrap_or(1) - 1;
                        let vt = p.next().flatten().unwrap_or(1) - 1;
                        let vn = p.next().flatten().unwrap_or(1) - 1;
                        Corner { v, vt, vn }
                    })
                    .collect();
                for i in 1..corners.len().saturating_sub(1) {
                    m.tris.push([corners[0], corners[i], corners[i + 1]]);
                }
            }
            _ => {}
        }
    }
    if m.tris.is_empty() {
        return Err(format!("{path}: sin caras"));
    }
    Ok(m)
}
