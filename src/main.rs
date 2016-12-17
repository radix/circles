// apparently I will be able to add this soon to make this app not open a console window on start
// #![windows_subsystem = "Windows"]
extern crate piston_window;
extern crate ncollide;
extern crate nalgebra as na;
extern crate find_folder;
extern crate fps_counter;
extern crate image as im; // "image" conflicts with something from piston_window

use piston_window::*;
mod space;
mod calc;
mod game;
mod render;

use game::*;
use calc::pt;

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Circles", [1024, 768])
        .exit_on_esc(true)
        .vsync(true)
        // .samples(8)
        .build()
        .unwrap();
    window.set_max_fps(60);
    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let ref font = assets.join("FiraSans-Regular.ttf");
    let factory = window.factory.clone();
    // I *could* make glyphs a part of App, but then self would have to be mut and I don't want to
    // do that.
    let mut glyphs: Glyphs = Glyphs::new(font, factory).unwrap();

    let mut app = App::new(&mut window);

    let mut input = GameInput::new();
    let mut cursor: Option<[f64; 2]> = None;
    let mut shooting = false;

    let mut fps_counter = fps_counter::FPSCounter::new();
    while let Some(e) = window.next() {
        cursor = match e.mouse_cursor_args() {
            None => cursor,
            x => x,
        };
        if let Some(press) = e.press_args() {
            match press {
                Button::Keyboard(key) => {
                    match key {
                        Key::Left | Key::A => input.left = true,
                        Key::Right | Key::E | Key::D => input.right = true,
                        Key::Up | Key::Comma | Key::W => input.up = true,
                        Key::Down | Key::O | Key::S => input.down = true,
                        Key::Space => input.jump = true,
                        x => println!("Keyboard Key {:?}", x),
                    }
                }
                Button::Mouse(key) => {
                    match key {
                        MouseButton::Left => shooting = true,
                        MouseButton::Right => input.attach = true,
                        x => println!("Mouse Key {:?}", x),
                    }
                }
                x => println!("Unknown Event {:?}", x),
            }
        }
        if let Some(press) = e.release_args() {
            match press {
                Button::Keyboard(key) => {
                    match key {
                        Key::B => input.toggle_debug = true,
                        Key::Left | Key::A => input.left = false,
                        Key::Right | Key::E | Key::D => input.right = false,
                        Key::Up | Key::Comma | Key::W => input.up = false,
                        Key::Down | Key::O | Key::S => input.down = false,
                        Key::Space => input.jump = false,
                        _ => {}
                    }
                }
                Button::Mouse(key) => {
                    match key {
                        MouseButton::Left => shooting = false,
                        MouseButton::Right => input.attach = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if let Some(u) = e.update_args() {
            // FIXME this shouldn't use app.camera_pos. basically input handling should be moved
            // somewhere else, like maybe into App
            input.shoot_target = if shooting {
                cursor.map(|c| pt(app.camera_pos.x + c[0], app.camera_pos.y + c[1]))
            } else {
                None
            };
            app.update(&u, &mut window, &input);
            if input.attach {
                input.attach = false;
            }
            if input.toggle_debug {
                input.toggle_debug = false;
            }
        }
        app.render(&mut window, &e, &mut glyphs, &mut fps_counter);
    }
}
