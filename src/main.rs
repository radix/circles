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
use opengl_graphics::{GlGraphics, OpenGL};
use ncollide::query;
use ncollide::shape::Ball;
use na::{Isometry2, Vector2};

const SHIP_SIZE: f64 = 50.0;
const SPEED: f64 = 5.0;
const JUMP_SPEED: f64 = 3.0;
const AIR_CONTROL_MOD: f64 = 50.0;

#[derive(Debug)]
struct Planet {
    x: f64,
    y: f64,
    radius: f64,
}

#[derive(Debug)]
pub struct Bullet {
    x: f64,
    y: f64,
    dir: f64, // radians
    speed: f64,
}

#[derive(Debug)]
pub struct App {
    rotation: f64, // ship rotation (position on the planet)
    jumping: bool,
    height: f64,
    planets: Vec<Planet>,
    attached_planet: usize,
    closest_planet_coords: (f64, f64),
    bullets: Vec<Bullet>,
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

        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let square = graphics::rectangle::square(0.0, 0.0, SHIP_SIZE);

        let (x, y) = self.ship_position();
        gl.draw(args.viewport(), |c, gl| {
            clear(BLACK, gl);
            let ship_transform = c.transform
                .trans(x, y)
                .rot_rad(self.rotation)
                .trans(-(SHIP_SIZE / 2.0), -(SHIP_SIZE / 2.0));
            graphics::rectangle(RED, square, ship_transform, gl);
            for planet in &self.planets {
                let circle = graphics::ellipse::circle(0.0, 0.0, planet.radius);
                graphics::ellipse(BLUE, circle, c.transform.trans(planet.x, planet.y), gl);
            }
            let line = [x, y, self.closest_planet_coords.0, self.closest_planet_coords.1];
            graphics::line(GREEN, 1.0, line, c.transform.trans(0.0, 0.0), gl);
        });
    }

    fn update(&mut self, args: &UpdateArgs, up: bool, _down: bool, left: bool, right: bool) {
        if !self.jumping {
            if left {
                self.rotation -= SPEED * args.dt;
            }
            if right {
                self.rotation += SPEED * args.dt;
            }
            if up {
                self.jumping = true;
            }
        }
        if self.jumping {
            self.height += JUMP_SPEED;
            if left {
                self.rotation -= SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
            }
            if right {
                self.rotation += SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
            }
        }

        let ship_ball = Ball::new(SHIP_SIZE);
        let (ship_x, ship_y) = self.ship_position();
        let ship_pos = Isometry2::new(Vector2::new(ship_x, ship_y), na::zero());
        let mut closest_planet_distance = self.height;
        for (planet_index, planet) in (&self.planets).iter().enumerate() {
            let planet_ball = Ball::new(planet.radius);
            let planet_pos = Isometry2::new(Vector2::new(planet.x, planet.y), na::zero());

            // Check if this is the closest planet
            let distance = query::distance(&ship_pos, &ship_ball, &planet_pos, &planet_ball);
            if distance < closest_planet_distance {
                closest_planet_distance = distance;
                self.closest_planet_coords = (planet.x, planet.y);
            }

            if planet_index == self.attached_planet {
                continue;
            }
            // Check if we've collided with the planet
            let collided = query::contact(&ship_pos, &ship_ball, &planet_pos, &planet_ball, 0.0);
            if let Some(_) = collided {
                // We are landing on a new planet
                self.attached_planet = planet_index;
                self.jumping = false;
                self.height = planet.radius + (SHIP_SIZE / 2.0);
                self.rotation = (ship_y - planet.y).atan2(ship_x - planet.x);
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
    let mut window: Window = WindowSettings::new("spinning-square", [1024, 768])
        .opengl(opengl)
        .exit_on_esc(true)
        .vsync(true)
        .samples(8)
        .build()
        .unwrap();

    // Create a new game and run it.
    let mut gl = GlGraphics::new(opengl);
    let mut app = App {
        jumping: false,
        height: 75.0,
        rotation: 0.0,
        attached_planet: 0,
        closest_planet_coords: (500.0, 500.0),
        bullets: vec![],
        planets: vec![Planet {
                          x: 500.0,
                          y: 500.0,
                          radius: 50.0,
                      },
                      Planet {
                          x: 800.0,
                          y: 300.0,
                          radius: 35.0,
                      },
                      Planet {
                          x: 300.0,
                          y: 200.0,
                          radius: 100.0,
                      }],
    };

    let mut events = window.events().max_fps(1000);
    while let Some(e) = events.next(&mut window) {
        if let Some(r) = e.render_args() {
            app.render(&mut gl, &r);
        }
        if let Some(u) = e.update_args() {
            app.update(&u, up, down, left, right);
        }
        if let Some(Button::Keyboard(key)) = e.press_args() {
            match key {
                Key::Left | Key::A => left = true,
                Key::Right | Key::E => right = true,
                Key::Up | Key::Comma => up = true,
                Key::Down | Key::O => down = true,
                _ => {}
            }
        }
        if let Some(Button::Keyboard(key)) = e.release_args() {
            match key {
                Key::Left | Key::A => left = false,
                Key::Right | Key::E => right = false,
                Key::Up | Key::Comma => up = false,
                Key::Down | Key::O => down = false,
                _ => {}
            }
        }
    }
}
