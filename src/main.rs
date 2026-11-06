// Lab 6 — planetas y estrellas con shaders procedurales en CPU.
mod math;
mod noise;
mod obj;
mod png;
mod render;
mod shaders;

use math::vec3;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use render::{Camera, Framebuffer};
use shaders::{Body, Uniforms};
use std::time::Instant;

const WIDTH: usize = 400; // resolución del framebuffer (la ventana lo escala x2)
const HEIGHT: usize = 400;
const DT: f32 = 1.0 / 60.0; // el tiempo avanza una cantidad fija por frame
const CAM_DIST: f32 = 3.0;
const FOV: f32 = 40.0;
const DEFAULT_COVERAGE: f32 = 0.5;

struct Scene {
    body: Body,
    yaw: f32,
    pitch: f32,
    light_az: f32, // azimut de la luz (rad)
    coverage: f32,
    time: f32,
}

impl Scene {
    fn new(body: Body) -> Self {
        Scene { body, yaw: 0.0, pitch: 0.15, light_az: 0.9, coverage: DEFAULT_COVERAGE, time: 0.0 }
    }

    fn render(&self, fb: &mut Framebuffer, mesh: &obj::Mesh) {
        let cam = Camera::orbit(self.yaw, self.pitch, CAM_DIST, fb.height, FOV);
        let uni = Uniforms {
            time: self.time,
            light_dir: vec3(self.light_az.sin(), 0.25, self.light_az.cos()).normalize(),
            cloud_coverage: self.coverage,
        };
        render::render(fb, mesh, &cam, self.body, &uni);
    }
}

fn body_name(b: Body) -> &'static str {
    match b {
        Body::Star => "estrella",
        Body::Rocky => "planeta_rocoso",
        Body::GasGiant => "gigante_gaseoso",
        Body::Soccer => "futbol",
        Body::Basketball => "basquetbol",
        Body::Baseball => "beisbol",
    }
}

fn body_from_index(i: u32) -> Body {
    match i {
        2 => Body::Rocky,
        3 => Body::GasGiant,
        4 => Body::Soccer,
        5 => Body::Basketball,
        6 => Body::Baseball,
        _ => Body::Star,
    }
}

/// Modo sin ventana: `--body N --out f.png [--time s --yaw deg --pitch deg --light deg --cloud c]`
/// o `--frames N --dir carpeta` para una secuencia de PNG (para el GIF).
fn headless(args: &[String], mesh: &obj::Mesh) {
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let num = |k: &str, d: f32| get(k).and_then(|s| s.parse().ok()).unwrap_or(d);
    let mut sc = Scene::new(body_from_index(num("--body", 1.0) as u32));
    sc.time = num("--time", 0.0);
    sc.yaw = num("--yaw", 0.0).to_radians();
    sc.pitch = num("--pitch", 8.6).to_radians();
    sc.light_az = num("--light", 52.0).to_radians();
    sc.coverage = num("--cloud", DEFAULT_COVERAGE);
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);

    if let Some(n) = get("--frames").and_then(|s| s.parse::<u32>().ok()) {
        let dir = get("--dir").unwrap_or("assets/frames".into());
        std::fs::create_dir_all(&dir).unwrap();
        for i in 0..n {
            sc.render(&mut fb, mesh);
            png::save(&format!("{dir}/f{i:04}.png"), &fb.color, WIDTH, HEIGHT).unwrap();
            sc.time += 1.0 / 30.0 * num("--speed", 3.0);
            sc.yaw += (num("--spin", 0.0) / n as f32).to_radians();
        }
        println!("{n} frames en {dir}");
    } else {
        let out = get("--out").unwrap_or("capturas/headless.png".into());
        let t0 = Instant::now();
        sc.render(&mut fb, mesh);
        let ms = t0.elapsed().as_secs_f32() * 1000.0;
        png::save(&out, &fb.color, WIDTH, HEIGHT).unwrap();
        println!("{out} ({ms:.1} ms)");
    }
}

fn main() {
    let mesh = obj::load("assets/sphere.obj").unwrap_or_else(|e| {
        eprintln!("No se pudo cargar el modelo: {e}");
        std::process::exit(1);
    });
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--out" || a == "--frames") {
        return headless(&args, &mesh);
    }

    let mut window = Window::new(
        "Lab 6 — shaders procedurales",
        WIDTH,
        HEIGHT,
        WindowOptions { scale: Scale::X2, ..WindowOptions::default() },
    )
    .expect("no se pudo abrir la ventana");
    window.set_target_fps(60);

    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    let mut sc = Scene::new(Body::Star);
    sc.pitch = 8.6f32.to_radians();
    let (mut shots, mut frames) = (0, 0u32);
    let mut last = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        for (key, i) in [(Key::Key1, 1), (Key::Key2, 2), (Key::Key3, 3), (Key::Key4, 4), (Key::Key5, 5), (Key::Key6, 6)] {
            if window.is_key_pressed(key, KeyRepeat::No) {
                sc.body = body_from_index(i);
            }
        }
        let k = |key| window.is_key_down(key);
        let step = 0.03;
        if k(Key::Left) { sc.yaw -= step; }
        if k(Key::Right) { sc.yaw += step; }
        if k(Key::Up) { sc.pitch = (sc.pitch + step).min(1.4); }
        if k(Key::Down) { sc.pitch = (sc.pitch - step).max(-1.4); }
        if k(Key::Q) { sc.light_az -= step; }
        if k(Key::E) { sc.light_az += step; }
        if k(Key::Z) { sc.coverage = (sc.coverage - 0.01).max(0.0); }
        if k(Key::X) { sc.coverage = (sc.coverage + 0.01).min(1.0); }
        if window.is_key_pressed(Key::R, KeyRepeat::No) {
            sc = Scene::new(sc.body);
            sc.pitch = 8.6f32.to_radians();
        }

        sc.time += DT;
        sc.render(&mut fb, &mesh);

        if window.is_key_pressed(Key::P, KeyRepeat::No) {
            std::fs::create_dir_all("capturas").ok();
            let path = format!("capturas/{}_{shots:02}.png", body_name(sc.body));
            match png::save(&path, &fb.color, WIDTH, HEIGHT) {
                Ok(()) => println!("Captura guardada: {path}"),
                Err(e) => eprintln!("No se pudo guardar {path}: {e}"),
            }
            shots += 1;
        }

        frames += 1;
        if last.elapsed().as_secs_f32() >= 1.0 {
            window.set_title(&format!(
                "Lab 6 — {} | {frames} fps | nubes {:.0}%",
                body_name(sc.body),
                sc.coverage * 100.0
            ));
            frames = 0;
            last = Instant::now();
        }
        window.update_with_buffer(&fb.color, WIDTH, HEIGHT).unwrap();
    }
}
