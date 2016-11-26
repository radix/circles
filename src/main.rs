extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate time;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL };

const SPEED: f64 = 5.0 / 10000000.0;

pub struct App {
    gl: GlGraphics, // OpenGL drawing backend.
    rotation: f64,  // Rotation for the square.
    x: f64,
    y: f64,
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED:   [f32; 4] = [1.0, 0.0, 0.0, 1.0];

        let square = rectangle::square(0.0, 0.0, 50.0);
        let rotation = self.rotation;
        let (x, y) = (self.x, self.y);

        self.gl.draw(args.viewport(), |c, gl| {
            // Clear the screen.
            clear(GREEN, gl);

            let transform = c.transform.trans(x, y)
                                       .rot_rad(rotation)
                                       .trans(-25.0, -25.0);

            // Draw a box rotating around the middle of the screen.
            rectangle(RED, square, transform, gl);
        });
    }

    fn update(&mut self, args: &UpdateArgs) {
        // Rotate 2 radians per second.
        self.rotation += 2.0 * args.dt;
    }
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    let mut left = false;
    let mut right = false;
    let mut down = false;
    let mut up = false;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new(
            "spinning-square",
            [1024, 768]
        )
        .opengl(opengl)
        .exit_on_esc(true)
        .vsync(false)
        .build()
        .unwrap();

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        rotation: 0.0,
        x: 100.0,
        y: 100.0
    };

    let mut events = window.events().max_fps(1000);
    let mut last_time = time::precise_time_ns() as f64;
    while let Some(e) = events.next(&mut window) {
        let new_time = time::precise_time_ns() as f64;
        let time_delta = new_time - last_time;
        last_time = new_time;
        if let Some(r) = e.render_args() {
            app.render(&r);
        }
        if let Some(u) = e.update_args() {
            app.update(&u);
        }
        match e.press_args() {
            Some(Button::Keyboard(Key::Left)) => left = true,
            Some(Button::Keyboard(Key::Right)) => right = true,
            Some(Button::Keyboard(Key::Up)) => up = true,
            Some(Button::Keyboard(Key::Down)) => down = true,
            _ => {}
        }
        match e.release_args() {
            Some(Button::Keyboard(Key::Left)) => left = false,
            Some(Button::Keyboard(Key::Right)) => right = false,
            Some(Button::Keyboard(Key::Up)) => up = false,
            Some(Button::Keyboard(Key::Down)) => down = false,
            _ => {}
        }
        if up { app.y -= time_delta * SPEED; }
        if down { app.y += time_delta * SPEED; }
        if left { app.x -= time_delta * SPEED; }
        if right { app.x += time_delta * SPEED; }
    }
}
