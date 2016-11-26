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

#[derive(Debug, Clone)]
struct Planet {
    x: f64,
    y: f64,
}

pub struct App {
    gl: GlGraphics, // OpenGL drawing backend.
    rotation: f64,  // Rotation for the square.
    x: f64,
    y: f64,
    planets: Vec<Planet>,
    attached_planet: u32
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED:   [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE:  [f32; 4] = [0.0, 0.0, 1.0, 1.0];

        let square = rectangle::square(0.0, 0.0, 50.0);
        let circle = ellipse::circle(0.0, 0.0, 50.0);

        let rotation = self.rotation;
        let (x, y) = (self.x, self.y);

        // XXX: Do I really need to clone these planets?
        let planets = self.planets.clone();

        self.gl.draw(args.viewport(), |c, gl| {
            // Clear the screen.
            clear(GREEN, gl);

            let transform = c.transform.trans(x, y)
                                       .rot_rad(rotation)
                                       .trans(-25.0, -25.0);

            // Draw a box rotating around the middle of the screen.
            rectangle(RED, square, transform, gl);
            // XXX: I can't use `self.planets` here, because we're in a closure here? it actually
            // complains about self.gl being used. It seems like I *should* be able to refer to the
            // planets data without having to make an extra copy of it.
            for planet in planets {
                ellipse(BLUE, circle, c.transform.trans(planet.x, planet.y), gl);
            }
        });
    }

    fn update(&mut self, args: &UpdateArgs, up: bool, down: bool, left: bool, right: bool) {
        const SPEED: f64 = 1000.0;

        // Rotate 2 radians per second.
        self.rotation += 2.0 * args.dt;
        if up { self.y -= args.dt * SPEED; }
        if down { self.y += args.dt * SPEED; }
        if left { self.x -= args.dt * SPEED; }
        if right { self.x += args.dt * SPEED; }
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
        .vsync(true)
        .build()
        .unwrap();

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        rotation: 0.0,
        x: 100.0,
        y: 100.0,
        planets: vec![Planet{x: 500.0, y: 500.0}, Planet{x: 800.0, y: 300.0}],
        attached_planet: 0,
    };

    let mut events = window.events().max_fps(1000);
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&r);
        }
        if let Some(u) = e.update_args() {
            app.update(&u, up, down, left, right);
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
    }
}
