extern crate piston_window;
extern crate ncollide;
extern crate nalgebra as na;

use std::f64::consts::PI;
use piston_window::*;
use ncollide::query;
use ncollide::shape::Ball;
use na::{Isometry2, Vector2};

mod space;

use space::*;

const SHIP_SIZE: f64 = 50.0;
const SPEED: f64 = 5.0;
const JUMP_SPEED: f64 = 3.0;
const AIR_CONTROL_MOD: f64 = 50.0;
const BULLET_SPEED: f64 = 1000.0;
const BULLET_SIZE: f64 = 5.0;
const ACCELERATION: f64 = 5.0;
const FIRE_COOLDOWN: f64 = 0.1;

#[derive(Debug)]
pub struct App {
    rotation: f64, // ship rotation (position on the planet)
    ship_pos: Point, // redundant data, optimization
    jumping: bool,
    exit_speed: f64,
    height: f64,
    space: Space,
    // planets: HashMap<(i32, i32), Vec<Planet>>, // planets are divided up into areas 1024x768 in size.
    attached_planet: PlanetIndex,
    closest_planet_coords: Point, // redundant data, optimization
    // NES-style would be to make this a [(f64, f64); 3], so only three bullets can exist at once
    bullets: Vec<Bullet>,
    fire_cooldown: f64,
    camera_pos: Point,
}

impl App {
    fn render(&self, mut window: &mut PistonWindow, event: &Event) {
        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

        let square = rectangle::square(0.0, 0.0, SHIP_SIZE);

        window.draw_2d(event, |c, g| {
            let planets = self.space.get_nearby_planets();
            let camera = c.transform.trans(-self.camera_pos.x, -self.camera_pos.y);
            clear(BLACK, g);
            let ship_transform = camera.trans(self.ship_pos.x, self.ship_pos.y)
                .rot_rad(self.rotation)
                .trans(-(SHIP_SIZE / 2.0), -(SHIP_SIZE / 2.0));
            rectangle(RED, square, ship_transform, g);

            for (_, planet) in planets {
                let circle = ellipse::circle(0.0, 0.0, planet.radius);
                ellipse(BLUE, circle, camera.trans(planet.pos.x, planet.pos.y), g);
            }
            for bullet in self.bullets.iter() {
                let circle = ellipse::circle(0.0, 0.0, BULLET_SIZE);
                ellipse(RED, circle, camera.trans(bullet.pos.x, bullet.pos.y), g);
            }
            let nearest_beam = [self.ship_pos.x,
                                self.ship_pos.y,
                                self.closest_planet_coords.x,
                                self.closest_planet_coords.y];
            line(GREEN, 1.0, nearest_beam, camera.trans(0.0, 0.0), g);

            let attached = &self.space.get_planet(self.attached_planet);
            let attached_beam = [self.ship_pos.x, self.ship_pos.y, attached.pos.x, attached.pos.y];
            line(BLUE, 1.0, attached_beam, camera.trans(0.0, 0.0), g);
        });
    }

    fn update(&mut self,
              args: &UpdateArgs,
              view_size: Size,
              up: bool,
              down: bool,
              left: bool,
              right: bool,
              shoot_target: Option<Point>,
              attach: bool) {
        // self.space.realize();
        {
            let attached_planet = self.space.get_planet(self.attached_planet);
            self.ship_pos.x = attached_planet.pos.x + (self.rotation.cos() * self.height);
            self.ship_pos.y = attached_planet.pos.y + (self.rotation.sin() * self.height);

            if let Some(target) = shoot_target {
                if self.fire_cooldown <= 0.0 {
                    let angle = (target.y - self.ship_pos.y).atan2(target.x - self.ship_pos.x);
                    let bullet = Bullet {
                        pos: self.ship_pos,
                        dir: angle,
                        speed: BULLET_SPEED,
                    };
                    self.bullets.push(bullet);
                    self.fire_cooldown = FIRE_COOLDOWN;
                } else {
                    self.fire_cooldown -= args.dt;
                }
            }

            let mut cull_bullets = vec![];
            let mut cull_counter = 0;
            for (idx, bullet) in self.bullets.iter_mut().enumerate() {
                bullet.pos.x = bullet.pos.x + (bullet.speed * args.dt * bullet.dir.cos());
                bullet.pos.y = bullet.pos.y + (bullet.speed * args.dt * bullet.dir.sin());
                // kind of a dumb hack to determine when to cull bullets. It'd be better if we had the
                // current view's bounding box (+ margin). or, alternatively, give each bullet a TTL.
                if (bullet.pos.x - self.ship_pos.x).abs() > 5000.0 ||
                   (bullet.pos.y - self.ship_pos.y).abs() > 5000.0 {
                    // rejigger the index so when we delete the items they compensate for previous
                    // deletions
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
                    self.exit_speed = JUMP_SPEED;
                }
            } else {
                self.height += self.exit_speed;
                if left {
                    self.rotation -= SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
                }
                if right {
                    self.rotation += SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
                }
                if down {
                    self.exit_speed -= ACCELERATION * args.dt;
                }
                if up {
                    self.exit_speed += ACCELERATION * args.dt;
                }
            }

            let ship_ball = Ball::new(SHIP_SIZE / 2.0);
            let ship_pos = Isometry2::new(Vector2::new(self.ship_pos.x, self.ship_pos.y),
                                          na::zero());
            let mut closest_planet_distance = self.height;
            let mut closest_planet_idx: PlanetIndex = self.attached_planet;
            let mut closest_planet = attached_planet;

            for (planet_index, planet) in self.space.get_nearby_planets() {
                let planet_ball = Ball::new(planet.radius);
                let planet_pos = Isometry2::new(Vector2::new(planet.pos.x, planet.pos.y),
                                                na::zero());

                // Check if this is the closest planet
                let distance = query::distance(&ship_pos, &ship_ball, &planet_pos, &planet_ball);
                if distance < closest_planet_distance {
                    closest_planet_distance = distance;
                    closest_planet_idx = planet_index;
                    closest_planet = planet;
                    self.closest_planet_coords = Point::new(planet.pos.x, planet.pos.y);
                }
                let collided =
                    query::contact(&ship_pos, &ship_ball, &planet_pos, &planet_ball, -1.0);
                if let Some(_) = collided {
                    // We are landing on a new planet
                    self.attached_planet = planet_index;
                    self.jumping = false;
                    self.height = planet.radius + (SHIP_SIZE / 2.0);
                    self.rotation = (self.ship_pos.y - planet.pos.y)
                        .atan2(self.ship_pos.x - planet.pos.x);
                    self.exit_speed = 0.0;
                }
            }
            if attach {
                // should we disallow attaching to the same planet? it basically allows player to
                // hard-stop
                self.attached_planet = closest_planet_idx;
                self.exit_speed = 0.0;
                self.rotation = (self.ship_pos.y - self.closest_planet_coords.y)
                    .atan2(self.ship_pos.x - self.closest_planet_coords.x);
                self.height = closest_planet_distance + closest_planet.radius + (SHIP_SIZE / 2.0);
            }

            if self.rotation > PI {
                self.rotation -= 2.0 * PI
            }
            if self.rotation < -PI {
                self.rotation += 2.0 * PI
            }


            let x_margin = (1.0 / 4.0) * view_size.width as f64;
            let y_margin = (1.0 / 4.0) * view_size.height as f64;
            let view_width_with_margin = view_size.width as f64 * (3.0 / 4.0);
            let view_height_with_margin = view_size.height as f64 * (3.0 / 4.0);

            if self.ship_pos.x > self.camera_pos.x + view_width_with_margin {
                self.camera_pos.x += self.ship_pos.x - (self.camera_pos.x + view_width_with_margin);
            } else if self.ship_pos.x < self.camera_pos.x + x_margin {
                self.camera_pos.x += self.ship_pos.x - self.camera_pos.x - x_margin;
            }
            if self.ship_pos.y > self.camera_pos.y + view_height_with_margin {
                self.camera_pos.y += self.ship_pos.y -
                                     (self.camera_pos.y + view_height_with_margin);
            } else if self.ship_pos.y < self.camera_pos.y + y_margin {
                self.camera_pos.y += self.ship_pos.y - self.camera_pos.y - y_margin;
            }
        }
        self.space.focus(self.ship_pos);
    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Circles", [1024, 768])
        .exit_on_esc(true)
        .vsync(false)
        // .samples(8)
        .build()
        .unwrap();

    // Create a new game and run it.
    let space = Space::new();
    let attached_planet_idx = space.get_nearby_planets()[0].0;
    let mut app = App {
        camera_pos: Point::new(-0.0, -0.0),
        jumping: false,
        fire_cooldown: 0.0,
        exit_speed: 0.0,
        ship_pos: Point::new(0.0, 0.0),
        height: space.get_planet(attached_planet_idx).radius,
        rotation: 0.0,
        attached_planet: attached_planet_idx,
        closest_planet_coords: Point::new(500.0, 500.0),
        bullets: vec![],
        space: Space::new(),
    };
    // probably move this into a struct
    let mut left = false;
    let mut right = false;
    let mut down = false;
    let mut up = false;
    let mut cursor: Option<[f64; 2]> = None;
    let mut shooting: bool = false;
    let mut attach: bool = false;

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
                        MouseButton::Left => shooting = true,
                        MouseButton::Right => attach = true,
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
                Button::Mouse(key) => {
                    match key {
                        MouseButton::Left => shooting = false,
                        MouseButton::Right => attach = false,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        if let Some(u) = e.update_args() {
            let shoot_target = if shooting {
                cursor.map(|c| Point::new(app.camera_pos.x + c[0], app.camera_pos.y + c[1]))
            } else {
                None
            };
            app.update(&u,
                       window.size(),
                       up,
                       down,
                       left,
                       right,
                       shoot_target,
                       attach);
            if attach {
                attach = false;
            }
        }
        app.render(&mut window, &e);
    }
}
