extern crate piston;
extern crate graphics;
extern crate glutin_window;
extern crate opengl_graphics;
extern crate time;
extern crate ncollide;
extern crate nalgebra as na;

use piston::window::WindowSettings;
use piston::event_loop::*;
use piston::input::*;
use glutin_window::GlutinWindow as Window;
use opengl_graphics::{ GlGraphics, OpenGL };
use ncollide::query;
use ncollide::shape::Ball;
use na::{Isometry2, Vector2};

#[derive(Debug)]
struct Planet {
    x: f64,
    y: f64,
}

#[derive(Debug)]
pub struct App {
    rotation: f64,  // ship rotation (position on the planet)
    jumping: bool,
    height: f64,
    planets: Vec<Planet>,
    attached_planet: usize
}

impl App {
    fn ship_position(&self) -> (f64, f64) {
        let attached_planet = &self.planets[self.attached_planet];
        let x = attached_planet.x + (self.rotation.cos() * self.height);
        let y = attached_planet.y + (self.rotation.sin() * self.height);
        (x, y)
    }

    fn render(&mut self, mut gl: &mut GlGraphics, args: &RenderArgs) {
        use graphics::*;

        // const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED:   [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE:  [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let square = rectangle::square(0.0, 0.0, 50.0);
        let circle = ellipse::circle(0.0, 0.0, 50.0);

        let (x, y) = self.ship_position();
        gl.draw(args.viewport(), |c, gl| {
            // Clear the screen.
            clear(BLACK, gl);
            let transform = c.transform.trans(x, y)
                                       .rot_rad(self.rotation)
                                       .trans(-25.0, -25.0);
            rectangle(RED, square, transform, gl);
            for planet in &self.planets {
                ellipse(BLUE, circle, c.transform.trans(planet.x, planet.y), gl);
            }
        });
    }

    fn update(&mut self, args: &UpdateArgs, up: bool, _down: bool, left: bool, right: bool) {
        const SPEED: f64 = 5.0;
        if !self.jumping {
            if left { self.rotation -= SPEED * args.dt; }
            if right { self.rotation += SPEED * args.dt; }
            if up { self.jumping = true; }
        }
        if self.jumping {
            self.height += SPEED;
            if left { self.rotation -= SPEED / 10.0 * args.dt}
            if right { self.rotation += SPEED / 10.0 * args.dt}
        }

        // create ship ball
        let ship_ball = Ball::new(25.0);
        let (ship_x, ship_y) = self.ship_position();
        let ship_pos = Isometry2::new(Vector2::new(ship_x, ship_y), na::zero());
        for (planet_index, planet) in (&self.planets).iter().enumerate() {
            let planet_ball = Ball::new(50.0);
            let planet_pos = Isometry2::new(
                Vector2::new(planet.x, planet.y),
                na::zero());
            let collided = query::contact(&ship_pos, &ship_ball, &planet_pos, &planet_ball, 0.0);
            if let Some(_) = collided {
                self.attached_planet = planet_index;
                self.jumping = false;
                self.height = 85.0;
            }
        }
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
    let mut gl = GlGraphics::new(opengl);
    let mut app = App {
        jumping: false,
        height: 85.0,
        rotation: 0.0,
        planets: vec![Planet{x: 500.0, y: 500.0}, Planet{x: 800.0, y: 300.0}],
        attached_planet: 0,
    };

    let mut events = window.events().max_fps(1000);
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&mut gl, &r);
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
