extern crate piston_window;
extern crate time;
extern crate ncollide;
extern crate nalgebra as na;

use std::f64::consts::PI;
use piston_window::*;
use ncollide::query;
use ncollide::shape::Ball;
use na::{Isometry2, Vector2};

const SHIP_SIZE: f64 = 50.0;
const SPEED: f64 = 5.0;
const JUMP_SPEED: f64 = 3.0;
const AIR_CONTROL_MOD: f64 = 50.0;
const BULLET_SPEED: f64 = 1000.0;
const BULLET_SIZE: f64 = 5.0;

#[derive(Debug)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn new(x: f64, y: f64) -> Point {
        Point { x: x, y: y }
    }
}

#[derive(Debug)]
struct Planet {
    pos: Point,
    radius: f64,
}

#[derive(Debug)]
pub struct Bullet {
    pos: Point,
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
        let x = attached_planet.pos.x + (self.rotation.cos() * self.height);
        let y = attached_planet.pos.y + (self.rotation.sin() * self.height);
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
                ellipse(BLUE,
                        circle,
                        c.transform.trans(planet.pos.x, planet.pos.y),
                        g);
            }
            for bullet in &self.bullets {
                let circle = ellipse::circle(0.0, 0.0, BULLET_SIZE);
                ellipse(RED,
                        circle,
                        c.transform.trans(bullet.pos.x, bullet.pos.y),
                        g);
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
              shoot_target: Option<[f64; 2]>) {
        let (ship_x, ship_y) = self.ship_position();
        if let Some(target) = shoot_target {
            let angle = (target[1] - ship_y).atan2(target[0] - ship_x);
            let bullet = Bullet {
                pos: Point::new(ship_x, ship_y),
                dir: angle,
                speed: BULLET_SPEED,
            };
            self.bullets.push(bullet);
        }

        let mut cull_bullets = vec![];
        let mut cull_counter = 0;
        for (idx, bullet) in (&mut self.bullets).iter_mut().enumerate() {
            bullet.pos.x = bullet.pos.x + (bullet.speed * args.dt * bullet.dir.cos());
            bullet.pos.y = bullet.pos.y + (bullet.speed * args.dt * bullet.dir.sin());
            if (bullet.pos.x - ship_x).abs() > 5000.0 || (bullet.pos.y - ship_y).abs() > 5000.0 {
                cull_bullets.push(idx - cull_counter);
                cull_counter += 1;
            }
        }

        for cull_idx in cull_bullets {
            self.bullets.remove(cull_idx);
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
            let planet_pos = Isometry2::new(Vector2::new(planet.pos.x, planet.pos.y), na::zero());

            // Check if this is the closest planet
            let distance = query::distance(&ship_pos, &ship_ball, &planet_pos, &planet_ball);
            if distance < closest_planet_distance {
                closest_planet_distance = distance;
                self.closest_planet_coords = (planet.pos.x, planet.pos.y);
            }
            if planet_index != self.attached_planet {
                // Check if we've collided with the planet
                let collided =
                    query::contact(&ship_pos, &ship_ball, &planet_pos, &planet_ball, 0.0);
                if let Some(_) = collided {
                    // We are landing on a new planet
                    self.attached_planet = planet_index;
                    self.jumping = false;
                    self.height = planet.radius + (SHIP_SIZE / 2.0);
                    self.rotation = (ship_y - planet.pos.y).atan2(ship_x - planet.pos.x);
                }
            }
        }

        if self.rotation > PI {
            self.rotation -= 2.0 * PI
        }
        if self.rotation < -PI {
            self.rotation += 2.0 * PI
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
                          pos: Point::new(500.0, 500.0),
                          radius: 50.0,
                      },
                      Planet {
                          pos: Point::new(200.0, 800.0),
                          radius: 35.0,
                      },
                      Planet {
                          pos: Point::new(300.0, 200.0),
                          radius: 100.0,
                      }],
    };

    let mut cursor: Option<[f64; 2]> = None;
    let mut shoot_target: Option<[f64; 2]> = None;

    while let Some(e) = window.next() {
        cursor = match e.mouse_cursor_args() {
            None => cursor,
            x => x,
        };
        if let Some(press) = e.press_args() {
            match press {
                Button::Keyboard(key) => {
                    match key {
                        Key::Left | Key::A => left = true,
                        Key::Right | Key::E | Key::D => right = true,
                        Key::Up | Key::Comma | Key::W => up = true,
                        Key::Down | Key::O | Key::S => down = true,
                        x => println!("Keyboard Key {:?}", x),
                    }
                }
                Button::Mouse(key) => {
                    match key {
                        MouseButton::Left => {
                            shoot_target = cursor;
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
                        Key::Right | Key::E | Key::D => right = false,
                        Key::Up | Key::Comma | Key::W => up = false,
                        Key::Down | Key::O | Key::S => down = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if let Some(u) = e.update_args() {
            app.update(&u, up, down, left, right, shoot_target);
            if let Some(_) = shoot_target {
                // Only shoot once per click!
                shoot_target = None;
            }
        }
        app.render(&mut window, &e);
    }
}
