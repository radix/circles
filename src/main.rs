extern crate piston_window;
extern crate time;
extern crate ncollide;
extern crate nalgebra as na;

use piston_window::*;
use ncollide::query;
use ncollide::shape::Ball;
use na::{Isometry2, Vector2};

const SHIP_SIZE: f64 = 50.0;
const SPEED: f64 = 5.0;
const JUMP_SPEED: f64 = 3.0;
const AIR_CONTROL_MOD: f64 = 50.0;
const BULLET_SPEED: f64 = 5.0;

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

    fn render(&mut self, mut window: &mut PistonWindow, event: &Event) {
        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let square = rectangle::square(0.0, 0.0, SHIP_SIZE);

        let (x, y) = self.ship_position();
        window.draw_2d(event, |c, g| {
            clear(BLACK, g);
            let ship_transform = c.transform
                .trans(x, y)
                .rot_rad(self.rotation)
                .trans(-(SHIP_SIZE / 2.0), -(SHIP_SIZE / 2.0));
            rectangle(RED, square, ship_transform, g);
            for planet in &self.planets {
                let circle = ellipse::circle(0.0, 0.0, planet.radius);
                ellipse(BLUE, circle, c.transform.trans(planet.x, planet.y), g);
            }
            for bullet in &self.bullets {
                let circle = ellipse::circle(0.0, 0.0, 1.0);
                ellipse(RED, circle, c.transform.trans(bullet.x, bullet.y), g);
            }
            let beam = [x, y, self.closest_planet_coords.0, self.closest_planet_coords.1];
            line(GREEN, 1.0, beam, c.transform.trans(0.0, 0.0), g);
        });
    }

    fn update(&mut self,
              args: &UpdateArgs,
              up: bool,
              _down: bool,
              left: bool,
              right: bool,
              shoot: Option<[f64; 2]>) {
        let (ship_x, ship_y) = self.ship_position();
        if let Some(target) = shoot {
            let angle = (ship_y - target[0]).atan2(ship_x - target[1]);
            let bullet = Bullet {
                x: ship_x,
                y: ship_y,
                dir: angle,
                speed: BULLET_SPEED,
            };
            self.bullets.push(bullet);
        }
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
        } else {
            self.height += JUMP_SPEED;
            if left {
                self.rotation -= SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
            }
            if right {
                self.rotation += SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
            }
        }

        let ship_ball = Ball::new(SHIP_SIZE);
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
    let mut left = false;
    let mut right = false;
    let mut down = false;
    let mut up = false;

    let mut window: PistonWindow = WindowSettings::new("spinning-square", [1024, 768])
        .exit_on_esc(true)
        .vsync(true)
        .samples(8)
        .build()
        .unwrap();

    // Create a new game and run it.
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

    while let Some(e) = window.next() {
        let mut shoot: Option<[f64; 2]> = None;
        if let Some(press) = e.press_args() {
            match press {
                Button::Keyboard(key) => {
                    match key {
                        Key::Left | Key::A => left = true,
                        Key::Right | Key::E => right = true,
                        Key::Up | Key::Comma => up = true,
                        Key::Down | Key::O => down = true,
                        x => println!("Keyboard Key {:?}", x),
                    }
                }
                Button::Mouse(key) => {
                    match key {
                        MouseButton::Left => {
                            shoot = e.mouse_cursor_args();
                            println!("Got shoot event! {:?}", shoot);
                        }
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
                        Key::Left | Key::A => left = false,
                        Key::Right | Key::E => right = false,
                        Key::Up | Key::Comma => up = false,
                        Key::Down | Key::O => down = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if let Some(r) = e.render_args() {
            app.render(&mut window, &e);
        }
        if let Some(u) = e.update_args() {
            app.update(&u, up, down, left, right, shoot);
        }
    }
}
