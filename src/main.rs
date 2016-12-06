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
const SPEED: f64 = 5.0;
const JUMP_SPEED: f64 = 3.0;
const AIR_CONTROL_MOD: f64 = 50.0;
const BULLET_SPEED: f64 = 1000.0;
const BULLET_SIZE: f64 = 5.0;
const ACCELERATION: f64 = 5.0;
const FIRE_COOLDOWN: f64 = 0.1;

struct GameInput {
    toggle_debug: bool,
    left: bool,
    right: bool,
    down: bool,
    up: bool,
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
            shoot_target: None,
            attach: false,
        }
    }
}

#[derive(Debug)]
pub struct App {
    debug: bool,
    rotation: f64, // ship rotation / position along the orbit
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

impl App {
    fn debug_point(&self,
                   g: &mut G2d,
                   context: &piston_window::Context,
                   glyphs: &mut Glyphs,
                   color: [f32; 4],
                   transform: [[f64; 3]; 2],
                   point: Point) {
        if self.debug {
            // Draw the x/y coords of the ship
            text::Text::new_color(color, 20).draw(&format!("{:.0}/{:.0}", point.x, point.y),
                                                  glyphs,
                                                  &context.draw_state,
                                                  transform,
                                                  g);
        }
    }

    fn render(&self, mut window: &mut PistonWindow, event: &Event, glyphs: &mut Glyphs) {
        const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
        const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
        const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
        const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
        const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

        let square = rectangle::square(0.0, 0.0, SHIP_SIZE);
        let view_size = window.size();
        let ship_pos = self.space.get_focus();
        let planets = self.space.get_nearby_planets();
        let bullet_gfx = ellipse::circle(0.0, 0.0, BULLET_SIZE);

        window.draw_2d(event, |c, g| {
            let camera = c.transform.trans(-self.camera_pos.x, -self.camera_pos.y);
            clear(BLACK, g);
            let ship_transform = camera.trans(ship_pos.x, ship_pos.y)
                .rot_rad(self.rotation)
                .trans(-(SHIP_SIZE / 2.0), -(SHIP_SIZE / 2.0));
            rectangle(RED, square, ship_transform, g);
            self.debug_point(g, &c, glyphs, WHITE, ship_transform, ship_pos);

            for (_, planet) in planets {
                if circle_in_view(planet.pos, planet.radius, self.camera_pos, view_size) {
                    let circle = ellipse::circle(0.0, 0.0, planet.radius);
                    let planet_transform = camera.trans(planet.pos.x, planet.pos.y);
                    ellipse(BLUE, circle, planet_transform, g);
                    self.debug_point(g, &c, glyphs, WHITE, planet_transform, planet.pos);
                }
            }
            if circle_in_view(self.magic_planet, 100.0, self.camera_pos, view_size) {
                let planet_gfx = ellipse::circle(0.0, 0.0, 100.0);
                ellipse(RED,
                        planet_gfx,
                        camera.trans(self.magic_planet.x, self.magic_planet.y),
                        g);
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
            let attached_planet = self.space.get_planet(self.attached_planet);
            let ship_pos = pt(attached_planet.pos.x + (self.rotation.cos() * self.height),
                              attached_planet.pos.y + (self.rotation.sin() * self.height));
            if let Some(target) = input.shoot_target {
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
                    self.fire_cooldown -= args.dt;
                }
            }

            let mut cull_bullets = vec![];
            let mut cull_counter = 0;
            for (idx, bullet) in self.bullets.iter_mut().enumerate() {
                bullet.pos.x = bullet.pos.x + (bullet.speed * args.dt * bullet.dir.cos());
                bullet.pos.y = bullet.pos.y + (bullet.speed * args.dt * bullet.dir.sin());
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

            if !self.jumping {
                if input.left {
                    self.rotation -= SPEED * args.dt;
                }
                if input.right {
                    self.rotation += SPEED * args.dt;
                }
                if input.up {
                    self.jumping = true;
                    self.exit_speed = JUMP_SPEED;
                }
            } else {
                self.height += self.exit_speed;
                if input.left {
                    self.rotation -= SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
                }
                if input.right {
                    self.rotation += SPEED * (AIR_CONTROL_MOD / self.height) * args.dt
                }
                if input.down {
                    self.exit_speed -= ACCELERATION * args.dt;
                }
                if input.up {
                    self.exit_speed += ACCELERATION * args.dt;
                }
            }

            let ship_ball = Ball::new(SHIP_SIZE / 2.0);
            let na_ship_pos = Isometry2::new(Vector2::new(ship_pos.x, ship_pos.y), na::zero());
            let mut closest_planet_distance = self.height;
            let mut closest_planet_idx: PlanetIndex = self.attached_planet;
            let mut closest_planet = attached_planet;

            for (planet_index, planet) in self.space.get_nearby_planets() {
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
                    self.jumping = false;
                    self.height = planet.radius + (SHIP_SIZE / 2.0);
                    self.rotation = direction_from_to(planet.pos, ship_pos);
                    self.exit_speed = 0.0;
                    println!("Attached to planet: {:?} {:?}", planet_index, planet);
                }
            }
            if input.attach {
                // should we disallow attaching to the same planet? it basically allows player to
                // hard-stop
                self.attached_planet = closest_planet_idx;
                self.exit_speed = 0.0;
                self.rotation = (ship_pos.y - self.closest_planet_coords.y)
                    .atan2(ship_pos.x - self.closest_planet_coords.x);
                self.height = closest_planet_distance + closest_planet.radius + (SHIP_SIZE / 2.0);
            }

            // Put a bound on rotation. This may be pointless...?
            if self.rotation > PI {
                self.rotation -= 2.0 * PI
            }
            if self.rotation < -PI {
                self.rotation += 2.0 * PI
            }


            // Update the camera so that the ship stays in view with a margin around the screen.
            let x_margin = (1.0 / 4.0) * view_size.width as f64;
            let y_margin = (1.0 / 4.0) * view_size.height as f64;
            let view_width_with_margin = view_size.width as f64 * (3.0 / 4.0);
            let view_height_with_margin = view_size.height as f64 * (3.0 / 4.0);

            if ship_pos.x > self.camera_pos.x + view_width_with_margin {
                self.camera_pos.x += ship_pos.x - (self.camera_pos.x + view_width_with_margin);
            } else if ship_pos.x < self.camera_pos.x + x_margin {
                self.camera_pos.x += ship_pos.x - self.camera_pos.x - x_margin;
            }
            if ship_pos.y > self.camera_pos.y + view_height_with_margin {
                self.camera_pos.y += ship_pos.y - (self.camera_pos.y + view_height_with_margin);
            } else if ship_pos.y < self.camera_pos.y + y_margin {
                self.camera_pos.y += ship_pos.y - self.camera_pos.y - y_margin;
            }
            ship_pos
        };
        self.space.focus(ship_pos);
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
