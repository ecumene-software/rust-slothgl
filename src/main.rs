use crossterm::{cursor, Crossterm, InputEvent, KeyEvent, RawScreen};
use std::error::Error;
use std::f32;
use std::fs::OpenOptions;
use std::io::{stdout, Write};
use std::path::Path;
use std::time::Instant;

use nalgebra::Rotation3;

pub mod base;
pub use base::*;

pub mod inputs;
pub use inputs::*;

fn to_meshes(models: Vec<tobj::Model>, materials: Vec<tobj::Material>) -> Vec<SimpleMesh> {
    let mut meshes: Vec<SimpleMesh> = vec![];
    for model in models {
        meshes.push(model.mesh.to_simple_mesh_with_materials(&materials));
    }
    meshes
}

fn main() -> Result<(), Box<Error>> {
    let matches = cli_matches(); // Read command line arguments
    let mut mesh_queue: Vec<SimpleMesh> = vec![]; // A list of meshes to render
    for slice in matches.value_of("INPUT FILENAME").unwrap().split(' ') {
        let error = |s: &str, e: &str| -> Vec<SimpleMesh> {
            println!("filename: [{}] couldn't load, {}. {}", slice, s, e);
            vec![]
        };
        // Fill list with file inputs (Splits for spaces -> multiple files)
        let path = Path::new(slice);
        let mut meshes = match path.extension() {
            None => error("couldn't determine filename extension", ""),
            Some(ext) => match ext.to_str() {
                None => error("couldn't parse filename extension", ""),
                Some(extstr) => match &*extstr.to_lowercase() {
                    "obj" => match tobj::load_obj(&path) {
                        Err(e) => error("tobj couldnt load/parse OBJ", &e.to_string()),
                        Ok(present) => to_meshes(present.0, present.1),
                    },
                    "stl" => match OpenOptions::new().read(true).open(&path) {
                        Err(e) => error("STL load failed", &e.to_string()),
                        Ok(mut file) => match stl_io::read_stl(&mut file) {
                            Err(e) => error("stl_io couldnt parse STL", &e.to_string()),
                            Ok(stlio_mesh) => vec![stlio_mesh.to_simple_mesh()],
                        },
                    },
                    _ => error("unknown filename extension", ""),
                },
            },
        };
        mesh_queue.append(&mut meshes);
    }
    let mut speed: f32 = 1.0; // Default speed for the x-axis spinning
    let mut turntable = (0.0, 0.0, 0.0); // Euler rotation variables, quaternions aren't very user friendly
    if matches.is_present("turntable") {
        // Parse turntable speed
        speed = matches.value_of("turntable").unwrap().parse().unwrap();
    }
    if matches.is_present("rotation") {
        let value = matches.value_of("rotation").unwrap(); // Parse initial rotation
        if value.matches(' ').count() != 2 {
            // At least 3 inputs "x y z"
            panic!("Incorrect arguments in rotation. Format: x y z (in degrees)"); // Panic on not enough or too many
        }
        let rotation: Vec<&str> = value.split(' ').collect();
        turntable = (
            rotation[0].parse().unwrap(),
            rotation[1].parse().unwrap(),
            rotation[2].parse().unwrap(),
        );
    }
    let mut context: Context = Context::blank(); // The context holds the frame+z buffer, and the width and height
    let size: (u16, u16) = (0, 0); // This is the terminal size, it's used to check when a new context must be made

    let crossterm = Crossterm::new();
    #[allow(unused)]
    let screen = RawScreen::into_raw_mode();
    let input = crossterm.input();
    let mut stdin = input.read_async();
    let cursor = cursor();

    cursor.hide()?;

    let mut last_time; // Used in the variable time step
    loop {
        last_time = Instant::now();
        if let Some(b) = stdin.next() {
            match b {
                InputEvent::Keyboard(event) => match event {
                    KeyEvent::Char('q') => break,
                    _ => {}
                },
                _ => {}
            }
        }
        let rot =
            Rotation3::from_euler_angles(turntable.0, turntable.1, turntable.2).to_homogeneous();
        context.update(size, &mesh_queue)?; // This checks for if there needs to be a context update
        context.clear(); // This clears the z and frame buffer
        for mesh in &mesh_queue {
            // Render all in mesh queue
            draw_mesh(&mut context, &mesh, rot, default_shader); // Draw all meshes
        }
        context.flush()?; // This prints all framebuffer info (good for changing colors ;)
        stdout().flush().unwrap();
        let dt = Instant::now().duration_since(last_time).as_nanos() as f32 / 1_000_000_000.0;
        turntable.1 += (speed * dt) as f32; // Turns the turntable
    }

    cursor.show()?;
    Ok(())
}
