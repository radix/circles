extern crate piston_window;
extern crate ncollide;
extern crate nalgebra as na;
extern crate find_folder;

use std::f64::consts::PI;
use piston_window::*;
use ncollide::query;
use ncollide::shape::Ball;
use na::{Isometry2, Vector2};

mod space;

use space::*;

const SHIP_SIZE: f64 = 50.0;
const CRAWLER_SIZE: f64 = 25.0;
const CRAWLER_SPEED: f64 = 2.0;
const SPEED: f64 = 5.0;
const FLY_SPEED: f64 = 3.0;
const JUMP_SPEED: f64 = 20.0;
const AIR_CONTROL_MOD: f64 = 0.50;
const BULLET_SPEED: f64 = 1000.0;
const BULLET_SIZE: f64 = 5.0;
const ACCELERATION: f64 = 5.0;
const FIRE_COOLDOWN: f64 = 0.1;
const GRAVITY: f64 = 50.0;

struct GameInput {
    toggle_debug: bool,
    left: bool,
    right: bool,
    down: bool,
    up: bool,
    jump: bool,
    shoot_target: Option<Point>,
    attach: bool,
}

impl GameInput {
    fn new() -> GameInput {
        GameInput {
            toggle_debug: false,
            left: false,
            right: false,
            down: false,
            up: false,
            jump: false,
            shoot_target: None,
            attach: false,
        }
    }
}

#[derive(Debug)]
pub struct App {
    debug: bool,
    rotation: f64, // ship rotation / position along the orbit
    flying: bool,
    jumping: bool,
    exit_speed: f64,
    height: f64,
    space: Space,
    attached_planet: PlanetIndex,
    closest_planet_coords: Point, // redundant data, optimization
    // NES-style would be to make this a [(f64, f64); 3], so only three bullets can exist at once
    bullets: Vec<Bullet>,
    fire_cooldown: f64,
    camera_pos: Point,
    magic_planet: Point,
}

/// Check if a circle is in the current viewport.
fn circle_in_view(point: Point,
                  radius: f64,
                  camera: Point,
                  view_size: piston_window::Size)
                  -> bool {
    point.x + radius > camera.x && point.x - radius < camera.x + view_size.width as f64 ||
    point.y + radius > camera.y && point.y - radius < camera.y + view_size.height as f64
}

/// Get the direction (in Radians) from one point to another.
fn direction_from_to(from: Point, to: Point) -> f64 {
    (to.y - from.y).atan2(to.x - from.x)
}

const DOWN_RIGHT_RAD: f64 = PI / 4.0;
const UP_RIGHT_RAD: f64 = -DOWN_RIGHT_RAD;
const DOWN_LEFT_RAD: f64 = PI * (3.0 / 4.0);
const UP_LEFT_RAD: f64 = -DOWN_LEFT_RAD;


#[derive(Eq, PartialEq, Ord, PartialOrd, Debug)]
enum Cardinal4 {
    Up,
    Down,
    Left,
    Right,
}

fn quad_direction(radians: f64) -> Cardinal4 {
    if DOWN_RIGHT_RAD > radians && radians > UP_RIGHT_RAD {
        Cardinal4::Right
    } else if DOWN_LEFT_RAD > radians && radians > DOWN_RIGHT_RAD {
        Cardinal4::Down
    } else if UP_LEFT_RAD < radians && radians < UP_RIGHT_RAD {
        Cardinal4::Up
    } else {
        Cardinal4::Left
    }
}

fn rotated_position(origin: Point, rotation: f64, height: f64) -> Point {
    pt(origin.x + (rotation.cos() * height),
       origin.y + (rotation.sin() * height))
}

impl App {
    fn debug(&self,
             g: &mut G2d,
             context: &piston_window::Context,
             glyphs: &mut Glyphs,
             color: [f32; 4],
             transform: [[f64; 3]; 2],
             text: &str) {
        if self.debug {
            text::Text::new_color(color, 20).draw(text, glyphs, &context.draw_state, transform, g);
        }
    }

    fn render(&self, mut window: &mut PistonWindow, event: &Event, glyphs: &mut Glyphs) {
        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const DARKRED: [f32; 4] = [0.5, 0.0, 0.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

        let square = rectangle::square(0.0, 0.0, SHIP_SIZE);
        let view_size = window.size();
        let ship_pos = self.space.get_focus();
        let planets = self.space.get_nearby_planets();
        let bugs = self.space.get_nearby_bugs();
        let bullet_gfx = ellipse::circle(0.0, 0.0, BULLET_SIZE);

        window.draw_2d(event, |c, g| {
            let camera = c.transform.trans(-self.camera_pos.x, -self.camera_pos.y);
            clear(BLACK, g);
            let ship_transform = camera.trans(ship_pos.x, ship_pos.y)
                .rot_rad(self.rotation)
                .trans(-(SHIP_SIZE / 2.0), -(SHIP_SIZE / 2.0));
            rectangle(RED, square, ship_transform, g);
            self.debug(g,
                       &c,
                       glyphs,
                       WHITE,
                       ship_transform,
                       &format!("{} h={:.1}", ship_pos, self.height));

            for (pidx, planet) in planets {
                if circle_in_view(planet.pos, planet.radius, self.camera_pos, view_size) {
                    let circle = ellipse::circle(0.0, 0.0, planet.radius);
                    let planet_transform = camera.trans(planet.pos.x, planet.pos.y);
                    ellipse(BLUE, circle, planet_transform, g);
                    self.debug(g,
                               &c,
                               glyphs,
                               WHITE,
                               planet_transform,
                               &format!("({}, {}) {} r={:.1}",
                                        pidx.get_area().0,
                                        pidx.get_area().1,
                                        planet.pos,
                                        planet.radius));
                }
            }
            if circle_in_view(self.magic_planet, 100.0, self.camera_pos, view_size) {
                let planet_gfx = ellipse::circle(0.0, 0.0, 100.0);
                ellipse(RED,
                        planet_gfx,
                        camera.trans(self.magic_planet.x, self.magic_planet.y),
                        g);
            }

            for bug in bugs {
                let planet = self.space.get_planet(bug.attached);
                let bug_pos =
                    rotated_position(planet.pos, bug.rotation, planet.radius + CRAWLER_SIZE);
                if circle_in_view(bug_pos, CRAWLER_SIZE, self.camera_pos, view_size) {
                    let bug_gfx = ellipse::circle(0.0, 0.0, CRAWLER_SIZE);
                    ellipse(DARKRED, bug_gfx, camera.trans(bug_pos.x, bug_pos.y), g);
                }
            }
            for bullet in self.bullets.iter() {
                if circle_in_view(bullet.pos, BULLET_SIZE, self.camera_pos, view_size) {
                    ellipse(RED, bullet_gfx, camera.trans(bullet.pos.x, bullet.pos.y), g);
                }
            }
            let nearest_beam = [ship_pos.x,
                                ship_pos.y,
                                self.closest_planet_coords.x,
                                self.closest_planet_coords.y];
            line(GREEN, 1.0, nearest_beam, camera, g);

            {
                let direction = direction_from_to(ship_pos, self.magic_planet);
                let cardinal = quad_direction(direction);
                let width = view_size.width as f64;
                let height = view_size.height as f64;
                let hint_line = match cardinal {
                    Cardinal4::Up => [0.0, 5.0, width, 5.0],
                    Cardinal4::Right => [width - 2.5, 0.0, width - 2.5, height],
                    Cardinal4::Left => [2.5, 0.0, 2.5, height],
                    Cardinal4::Down => [0.0, height - 2.5, width, height - 2.5],
                };
                line(RED, 5.0, hint_line, c.transform, g);
            }
            let attached = &self.space.get_planet(self.attached_planet);
            let attached_beam = [ship_pos.x, ship_pos.y, attached.pos.x, attached.pos.y];
            line(BLUE, 1.0, attached_beam, camera, g);
        });
    }

    fn update(&mut self, args: &UpdateArgs, view_size: Size, input: &GameInput) {
        if input.toggle_debug {
            self.debug = !self.debug;
        }
        // this next little bit of scope weirdness is brought to you by mutable borrows being
        // confusing
        let ship_pos = {
            let ship_pos = {
                let attached_planet = self.space.get_planet(self.attached_planet);
                rotated_position(attached_planet.pos, self.rotation, self.height)
            };

            if let Some(target) = input.shoot_target {
                self.update_shoot(target, ship_pos, args.dt)
            }

            self.update_cull_bullets(ship_pos, args.dt);
            self.update_movement(input, args.dt);

            let ship_ball = Ball::new(SHIP_SIZE / 2.0);
            let na_ship_pos = Isometry2::new(Vector2::new(ship_pos.x, ship_pos.y), na::zero());
            let mut closest_planet_distance = self.height;
            let mut closest_planet_idx: PlanetIndex = self.attached_planet;
            let mut closest_planet = self.space.get_planet(self.attached_planet);
            let planets = self.space.get_nearby_planets();

            for (planet_index, planet) in planets {
                let planet_ball = Ball::new(planet.radius);
                let planet_pos = Isometry2::new(Vector2::new(planet.pos.x, planet.pos.y),
                                                na::zero());

                // Check if this is the closest planet
                let distance = query::distance(&na_ship_pos, &ship_ball, &planet_pos, &planet_ball);
                if distance < closest_planet_distance {
                    closest_planet_distance = distance;
                    closest_planet_idx = planet_index;
                    closest_planet = planet;
                    self.closest_planet_coords = pt(planet.pos.x, planet.pos.y);
                }
                let collided =
                    query::contact(&na_ship_pos, &ship_ball, &planet_pos, &planet_ball, -1.0);
                if let Some(_) = collided {
                    // We are landing on a new planet
                    self.attached_planet = planet_index;
                    self.flying = false;
                    self.jumping = false;
                    self.height = planet.radius + (SHIP_SIZE / 2.0);
                    self.rotation = direction_from_to(planet.pos, ship_pos);
                    self.exit_speed = 0.0;
                }
            }
            if input.attach && closest_planet_idx != self.attached_planet {
                self.attached_planet = closest_planet_idx;
                self.exit_speed = 0.0;
                self.rotation = (ship_pos.y - self.closest_planet_coords.y)
                    .atan2(ship_pos.x - self.closest_planet_coords.x);
                self.height = closest_planet_distance + closest_planet.radius + (SHIP_SIZE / 2.0);
            }
            ship_pos
        };

        // Put a bound on rotation. This may be pointless...?
        if self.rotation > PI {
            self.rotation -= 2.0 * PI
        }
        if self.rotation < -PI {
            self.rotation += 2.0 * PI
        }
        self.camera_pos = self.update_camera(view_size, ship_pos);

        self.update_bugs(args.dt);
        self.space.focus(ship_pos);
    }

    fn update_shoot(&mut self, target: Point, ship_pos: Point, time_delta: f64) {
        if self.fire_cooldown <= 0.0 {
            let angle = direction_from_to(ship_pos, target);
            let bullet = Bullet {
                pos: ship_pos,
                dir: angle,
                speed: BULLET_SPEED,
            };
            self.bullets.push(bullet);
            self.fire_cooldown = FIRE_COOLDOWN;
        } else {
            self.fire_cooldown -= time_delta;
        }
    }

    fn update_cull_bullets(&mut self, ship_pos: Point, time_delta: f64) {
        let mut cull_bullets = vec![];
        let mut cull_counter = 0;
        for (idx, bullet) in self.bullets.iter_mut().enumerate() {
            bullet.pos.x = bullet.pos.x + (bullet.speed * time_delta * bullet.dir.cos());
            bullet.pos.y = bullet.pos.y + (bullet.speed * time_delta * bullet.dir.sin());
            // kind of a dumb hack to determine when to cull bullets. It'd be better if we had
            // the current view's bounding box (+ margin). or, alternatively, give each bullet
            // a TTL.
            if (bullet.pos.x - ship_pos.x).abs() > 5000.0 ||
               (bullet.pos.y - ship_pos.y).abs() > 5000.0 {
                // rejigger the index so when we delete the items they compensate for previous
                // deletions
                cull_bullets.push(idx - cull_counter);
                cull_counter += 1;
            }
        }
        for cull_idx in cull_bullets {
            self.bullets.remove(cull_idx);
        }
    }

    fn update_movement(&mut self, input: &GameInput, time_delta: f64) {
        if !self.flying {
            if input.up {
                self.flying = true;
                self.exit_speed = FLY_SPEED;
            }
        } else {
            self.height += self.exit_speed;
            if input.down {
                self.exit_speed -= ACCELERATION * time_delta;
            }
            if input.up {
                self.exit_speed += ACCELERATION * time_delta;
            }
        }

        if !self.jumping {
            if input.jump {
                self.jumping = true;
                self.exit_speed = JUMP_SPEED;
            }
        } else {
            self.exit_speed -= GRAVITY * time_delta;
            self.height += self.exit_speed;
            if !input.jump {
                if self.exit_speed > (JUMP_SPEED / 2.0) {
                    self.exit_speed = JUMP_SPEED / 2.0;
                }
            }
        }

        if self.flying || self.jumping {
            if input.left {
                self.rotation -= SPEED * AIR_CONTROL_MOD * time_delta
            }
            if input.right {
                self.rotation += SPEED * AIR_CONTROL_MOD * time_delta
            }
        } else {
            if input.left {
                self.rotation -= SPEED * time_delta;
            }
            if input.right {
                self.rotation += SPEED * time_delta;
            }
        }
    }
    fn update_camera(&self, view_size: piston_window::Size, ship_pos: Point) -> Point {
        // Update the camera so that the ship stays in view with a margin around the screen.
        let x_margin = (1.0 / 4.0) * view_size.width as f64;
        let y_margin = (1.0 / 4.0) * view_size.height as f64;
        let view_width_with_margin = view_size.width as f64 * (3.0 / 4.0);
        let view_height_with_margin = view_size.height as f64 * (3.0 / 4.0);

        let x = if ship_pos.x > self.camera_pos.x + view_width_with_margin {
            self.camera_pos.x + ship_pos.x - (self.camera_pos.x + view_width_with_margin)
        } else if ship_pos.x < self.camera_pos.x + x_margin {
            self.camera_pos.x + ship_pos.x - self.camera_pos.x - x_margin
        } else {
            self.camera_pos.x
        };
        let y = if ship_pos.y > self.camera_pos.y + view_height_with_margin {
            self.camera_pos.y + ship_pos.y - (self.camera_pos.y + view_height_with_margin)
        } else if ship_pos.y < self.camera_pos.y + y_margin {
            self.camera_pos.y + ship_pos.y - self.camera_pos.y - y_margin
        } else {
            self.camera_pos.y
        };
        pt(x, y)
    }

    fn update_bugs(&mut self, time_delta: f64) {
        let shot_crawlers = {
            let bball = Ball::new(BULLET_SIZE);
            let crawler_ball = Ball::new(CRAWLER_SIZE);
            for crawler in self.space.get_nearby_bugs_mut() {
                crawler.rotation -= CRAWLER_SPEED * time_delta;
            }
            let mut shot_crawlers = vec![];
            for crawler in self.space.get_nearby_bugs() {
                let planet = self.space.get_planet(crawler.attached);
                let bug_pos =
                    rotated_position(planet.pos, crawler.rotation, planet.radius + CRAWLER_SIZE);
                let crawler_pos = Isometry2::new(Vector2::new(bug_pos.x, bug_pos.y), na::zero());

                for bullet in self.bullets.iter() {
                    let bpos = Isometry2::new(Vector2::new(bullet.pos.x, bullet.pos.y), na::zero());
                    let collided = query::contact(&crawler_pos, &crawler_ball, &bpos, &bball, 0.0);
                    if let Some(_) = collided {
                        shot_crawlers.push(crawler.attached);
                    }
                }
            }
            shot_crawlers
        };
        for p in shot_crawlers {
            self.space.delete_bug(p);
        }

    }
}

fn main() {
    let mut window: PistonWindow = WindowSettings::new("Circles", [1024, 768])
        .exit_on_esc(true)
        .vsync(false)
        // .samples(8)
        .build()
        .unwrap();

    let assets = find_folder::Search::ParentsThenKids(3, 3)
        .for_folder("assets")
        .unwrap();
    let ref font = assets.join("FiraSans-Regular.ttf");
    let factory = window.factory.clone();
    let mut glyphs: Glyphs = Glyphs::new(font, factory).unwrap();

    // Create a new game and run it.
    let space = Space::new();
    let attached_planet_idx = space.get_nearby_planets()[0].0;
    let mut app = App {
        debug: false,
        camera_pos: pt(-0.0, -0.0),
        flying: false,
        jumping: false,
        fire_cooldown: 0.0,
        exit_speed: 0.0,
        height: space.get_planet(attached_planet_idx).radius,
        rotation: 0.0,
        attached_planet: attached_planet_idx,
        closest_planet_coords: pt(500.0, 500.0),
        bullets: vec![],
        space: Space::new(),
        magic_planet: pt(500.0, 500.0),
    };
    let mut input = GameInput::new();
    let mut cursor: Option<[f64; 2]> = None;
    let mut shooting = false;
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
            input.shoot_target = if shooting {
                cursor.map(|c| pt(app.camera_pos.x + c[0], app.camera_pos.y + c[1]))
            } else {
                None
            };
            app.update(&u, window.size(), &input);
            if input.attach {
                input.attach = false;
            }
            if input.toggle_debug {
                input.toggle_debug = false;
            }
        }
        app.render(&mut window, &e, &mut glyphs);
    }
}
