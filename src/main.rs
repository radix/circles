// apparently I will be able to add this soon to make this app not open a console window on start
// #![windows_subsystem = "Windows"]
extern crate piston_window;
extern crate ncollide;
extern crate nalgebra as na;
extern crate find_folder;
extern crate fps_counter;
extern crate image as im; // "image" conflicts with something from piston_window

use piston_window::{PistonWindow, WindowSettings, Glyphs, EventLoop, UpdateEvent};
mod space;
mod calc;
mod game;
mod render;

use game::App;

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

    let mut fps_counter = fps_counter::FPSCounter::new();
    while let Some(e) = window.next() {
        app.gather_input(&e);
        if let Some(u) = e.update_args() {
            app.update(&u, &mut window);
        }
        app.render(&mut window, &e, &mut glyphs, &mut fps_counter);
    }
}
