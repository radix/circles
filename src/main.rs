// apparently I will be able to add this soon to make this app not open a console window on start
// #![windows_subsystem = "Windows"]
extern crate piston_window;
extern crate ncollide;
extern crate nalgebra as na;
extern crate find_folder;
extern crate fps_counter;
extern crate image as im;

use std::f64::consts::PI;
use piston_window::*;
use ncollide::query;
use ncollide::shape::Ball;

mod space;
mod calc;

use space::*;
use calc::*;

const SHIP_SIZE: f64 = 50.0;
const CRAWLER_SIZE: f64 = 25.0;
const CRAWLER_SPEED: f64 = 2.0;
const SPEED: f64 = 5.0;
const FLY_SPEED: f64 = 3.0;
const JUMP_SPEED: f64 = 10.0;
const AIR_CONTROL_MOD: f64 = 0.40;
const BULLET_SPEED: f64 = 1000.0;
const BULLET_SIZE: f64 = 5.0;
const ACCELERATION: f64 = 5.0;
const FIRE_COOLDOWN: f64 = 0.1;
const GRAVITY: f64 = 20.0;
const MAGIC_PLANET_SIZE: f64 = 200.0;
const MINI_WIDTH: f64 = 200.0;
const MINI_HEIGHT: f64 = 200.0;

const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const DARKRED: [f32; 4] = [0.5, 0.0, 0.0, 1.0];
const LIGHTBLUE: [f32; 4] = [0.5, 0.5, 1.0, 1.0];
const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

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


type Transform = [[f64; 3]; 2];


/// Check if a circle is in the current viewport.
fn circle_in_view(point: Point, radius: f64, camera: Point, view_size: Size) -> bool {
    point.x + radius > camera.x && point.x - radius < camera.x + view_size.width as f64 ||
    point.y + radius > camera.y && point.y - radius < camera.y + view_size.height as f64
}


pub struct App {
    // meta-state? or something
    debug: bool,
    // rendering state
    // glyphs: Glyphs
    minimap: G2dTexture,
    space_bounds: (Point, Point), // min and max
    // gameplay state
    space: Space,
    score: i8,
    rotation: f64, // ship rotation / position along the orbit
    flying: bool,
    jumping: bool,
    exit_speed: f64,
    height: f64,
    attached_planet: PlanetIndex,
    closest_planet_coords: Point, // redundant data, optimization
    // NES-style would be to make this a [(f64, f64); 3], so only three bullets can exist at once
    bullets: Vec<Bullet>,
    fire_cooldown: f64,
    camera_pos: Point,
}

impl App {
    fn debug(&self,
             g: &mut G2d,
             context: &Context,
             glyphs: &mut Glyphs,
             color: [f32; 4],
             transform: Transform,
             text: &str) {
        if self.debug {
            text::Text::new_color(color, 20).draw(text, glyphs, &context.draw_state, transform, g);
        }
    }

    fn render(&self,
              mut window: &mut PistonWindow,
              event: &Event,
              glyphs: &mut Glyphs,
              fps_counter: &mut fps_counter::FPSCounter) {

        let view_size = window.size();

        window.draw_2d(event, |c, g| {
            let ship_pos = self.space.get_focus();
            let planets = self.space.get_nearby_planets();
            let bugs = self.space.get_nearby_bugs();
            let bullet_gfx = rectangle::square(-BULLET_SIZE / 2.0, -BULLET_SIZE / 2.0, BULLET_SIZE);
            let camera = c.transform.trans(-self.camera_pos.x, -self.camera_pos.y);
            clear(BLACK, g);

            let nearest_beam = [ship_pos.x,
                                ship_pos.y,
                                self.closest_planet_coords.x,
                                self.closest_planet_coords.y];
            line(GREEN, 1.0, nearest_beam, camera, g);

            // Draw the attached beam
            let attached = &self.space.get_planet(self.attached_planet);
            let attached_beam = [ship_pos.x, ship_pos.y, attached.pos.x, attached.pos.y];
            line(BLUE, 1.0, attached_beam, camera, g);

            self.render_ship(glyphs, ship_pos, camera, &c, g);
            self.render_planets(glyphs, planets, camera, &c, g, view_size);
            self.render_bugs(bugs, camera, g, view_size);
            self.render_bullets(bullet_gfx, camera, g, view_size);
            let fps = fps_counter.tick();
            if self.debug {
                self.render_fps(glyphs, fps, &c, g);
            }

            self.render_score(glyphs, &c, g);
            self.render_minimap(&c, g);
            self.render_hint(ship_pos, camera, g);
        });
    }

    fn render_minimap(&self, context: &Context, g: &mut G2d) {
        let trans = context.transform.trans(50.0, 50.0);
        image(&self.minimap, trans, g);
        // draw a dot representing the ship
        {
            let (mini_x, mini_y) = shrink_to_bounds(MINI_WIDTH,
                                                    MINI_HEIGHT,
                                                    self.space_bounds.0,
                                                    self.space_bounds.1,
                                                    self.space.get_focus());
            rectangle(RED,
                      rectangle::square(0.0, 0.0, 1.0),
                      trans.trans(mini_x as f64, mini_y as f64),
                      g);
        }
        {
            let r =
                rectangle::rectangle_by_corners(-1.0, -1.0, MINI_WIDTH + 1.0, MINI_HEIGHT + 1.0);
            Rectangle::new_border(WHITE, 1.0).draw(r, &context.draw_state, trans, g);
        }
    }

    fn render_fps(&self, glyphs: &mut Glyphs, fps: usize, context: &Context, g: &mut G2d) {
        text::Text::new_color(WHITE, 20).draw(&format!("FPS: {}", fps),
                                              glyphs,
                                              &context.draw_state,
                                              context.transform.trans(100.0, 100.0),
                                              g);
    }

    fn render_score(&self, glyphs: &mut Glyphs, context: &Context, g: &mut G2d) {
        text::Text::new_color(WHITE, 20).draw(&format!("{}", self.score),
                                              glyphs,
                                              &context.draw_state,
                                              context.transform.trans(20.0, 20.0),
                                              g);
    }

    fn render_ship(&self,
                   glyphs: &mut Glyphs,
                   ship_pos: Point,
                   camera: Transform,
                   context: &Context,
                   g: &mut G2d) {
        let square = rectangle::square(0.0, 0.0, SHIP_SIZE);
        let ship_transform = camera.trans(ship_pos.x, ship_pos.y)
            .rot_rad(self.rotation)
            .trans(-(SHIP_SIZE / 2.0), -(SHIP_SIZE / 2.0));
        rectangle(RED, square, ship_transform, g);
        self.debug(g,
                   &context,
                   glyphs,
                   WHITE,
                   ship_transform,
                   &format!("{} h={:.1}", ship_pos, self.height));
    }

    fn render_planets(&self,
                      glyphs: &mut Glyphs,
                      planets: Vec<(PlanetIndex, &Planet)>,
                      camera: Transform,
                      context: &Context,
                      g: &mut G2d,
                      view_size: Size) {
        for (pidx, planet) in planets {
            if circle_in_view(planet.pos, planet.radius, self.camera_pos, view_size) {
                let planet_transform = camera.trans(planet.pos.x, planet.pos.y);
                let color = if planet.bouncy { LIGHTBLUE } else { BLUE };
                if !self.debug {
                    let circle = ellipse::circle(0.0, 0.0, planet.radius);
                    ellipse(color, circle, planet_transform, g);
                } else {
                    let side = (planet.radius * 2.0) / ((2.0 as f64).sqrt());
                    let circle = rectangle::square(-side / 2.0, -side / 2.0, side);
                    rectangle(color, circle, planet_transform, g);
                }
                self.debug(g,
                           &context,
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
        if circle_in_view(self.space.get_magic_planet(),
                          MAGIC_PLANET_SIZE,
                          self.camera_pos,
                          view_size) {
            let planet_gfx = ellipse::circle(0.0, 0.0, MAGIC_PLANET_SIZE);
            ellipse(RED,
                    planet_gfx,
                    camera.trans(self.space.get_magic_planet().x,
                                 self.space.get_magic_planet().y),
                    g);
        }
    }

    fn render_bugs(&self,
                   bugs: Vec<(Area, usize)>,
                   camera: Transform,
                   g: &mut G2d,
                   view_size: Size) {
        for (area, idx) in bugs {
            let bug = self.space.get_bug(area, idx);
            let planet = self.space.get_planet(bug.attached);
            let bug_pos = rotated_position(planet.pos, bug.rotation, planet.radius + CRAWLER_SIZE);
            if circle_in_view(bug_pos, CRAWLER_SIZE, self.camera_pos, view_size) {
                let bug_gfx = ellipse::circle(0.0, 0.0, CRAWLER_SIZE);
                ellipse(DARKRED, bug_gfx, camera.trans(bug_pos.x, bug_pos.y), g);
            }
        }
    }

    fn render_bullets(&self,
                      bullet_gfx: [f64; 4],
                      camera: Transform,
                      g: &mut G2d,
                      view_size: Size) {
        for bullet in self.bullets.iter() {
            if circle_in_view(bullet.pos, BULLET_SIZE, self.camera_pos, view_size) {
                rectangle(RED, bullet_gfx, camera.trans(bullet.pos.x, bullet.pos.y), g);
            }
        }
    }

    /// Draw the hint towards the magic planet
    fn render_hint(&self, ship_pos: Point, camera: Transform, g: &mut G2d) {
        const DOWN_RIGHT_RAD: f64 = PI / 4.0;
        const UP_RIGHT_RAD: f64 = -DOWN_RIGHT_RAD;
        const DOWN_LEFT_RAD: f64 = PI * (3.0 / 4.0);
        const UP_LEFT_RAD: f64 = -DOWN_LEFT_RAD;

        let magic_dir = direction_from_to(ship_pos, self.space.get_magic_planet());

        let dot = rectangle::square(-2.5, -2.5, 5.0);
        for rot in [DOWN_RIGHT_RAD, UP_RIGHT_RAD, DOWN_LEFT_RAD, UP_LEFT_RAD].iter() {
            let corner_rot = rot + self.rotation;
            if self.debug {
                let corner = rotated_position(ship_pos, corner_rot, SHIP_SIZE / 2.0);
                rectangle(BLUE, dot, camera.trans(corner.x, corner.y), g);
            }
        }

        let hint_rad = if DOWN_RIGHT_RAD + self.rotation > magic_dir &&
                          magic_dir > UP_RIGHT_RAD + self.rotation {
            self.rotation
        } else if DOWN_LEFT_RAD + self.rotation > magic_dir &&
                                 magic_dir > DOWN_RIGHT_RAD + self.rotation {
            self.rotation + PI / 2.0
        } else if UP_LEFT_RAD + self.rotation < magic_dir &&
                                 magic_dir < UP_RIGHT_RAD + self.rotation {
            self.rotation - PI / 2.0
        } else {
            self.rotation - PI
        };
        let hint_pos = rotated_position(ship_pos, hint_rad, 30.0);
        ellipse(GREEN, dot, camera.trans(hint_pos.x, hint_pos.y), g);
    }

    fn update(&mut self, args: &UpdateArgs, window: &mut PistonWindow, input: &GameInput) {
        // annoyed that I need the whole mutable window for this function. Only because it's
        // necessary to create a texture.
        let view_size = window.size();
        if input.toggle_debug {
            self.debug = !self.debug;
        }
        let ship_pos = {
            let attached_planet = self.space.get_planet(self.attached_planet);
            rotated_position(attached_planet.pos, self.rotation, self.height)
        };

        // It would be nice if more of these methods took &self instead of &mut self, and we
        // assigned the results
        if let Some(target) = input.shoot_target {
            self.update_shoot(target, ship_pos, args.dt)
        }

        self.update_bullets(ship_pos, args.dt);
        self.update_movement(input, args.dt);
        let (closest_planet_idx, closest_planet_distance) = self.update_collision(window, ship_pos);
        self.update_attach(closest_planet_idx, closest_planet_distance, ship_pos, input);

        // Put a bound on rotation, because maybe something bad will happen if someone spins in one
        // direction for an hour
        if self.rotation > PI {
            self.rotation -= 2.0 * PI
        }
        if self.rotation < -PI {
            self.rotation += 2.0 * PI
        }
        let on_planet = if self.flying || self.jumping {
            None
        } else {
            Some(self.space.get_planet(self.attached_planet).pos)
        };
        self.camera_pos = self.update_camera(view_size, on_planet, self.camera_pos, ship_pos);

        self.update_bugs(window, ship_pos, args.dt);
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

    fn update_bullets(&mut self, ship_pos: Point, time_delta: f64) {
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

    fn update_reset(&mut self, window: &mut PistonWindow, won: bool) {
        self.score += if won { 1 } else { -1 };
        self.space = Space::new();
        let attached_planet_idx = self.space.get_first_planet();
        self.attached_planet = attached_planet_idx;
        let attached_planet = self.space.get_planet(attached_planet_idx);
        self.height = attached_planet.radius;
        self.closest_planet_coords = attached_planet.pos;
        self.bullets = vec![];
        self.flying = false;
        self.jumping = false;
        self.fire_cooldown = 0.0;
        self.exit_speed = 0.0;
        self.rotation = 0.0;
        self.space_bounds = self.space.get_space_bounds();
        self.minimap = generate_minimap(window, &self.space, self.space_bounds);
    }

    /// Update game state based on collision.
    /// Returns the closest planet and the distance to it (for use in attachment).
    fn update_collision(&mut self,
                        window: &mut PistonWindow,
                        ship_pos: Point)
                        -> (PlanetIndex, f64) {
        let ship_ball = Ball::new(SHIP_SIZE / 2.0);
        let na_ship_pos = coll_pt(ship_pos);
        let mut closest_planet_distance = self.height;
        let mut closest_planet_idx: PlanetIndex = self.attached_planet;

        // check if the player found the magic planet
        {
            let planet_ball = Ball::new(MAGIC_PLANET_SIZE);
            let planet_pos = coll_pt(self.space.get_magic_planet());
            if let Some(_) = query::contact(&na_ship_pos,
                                            &ship_ball,
                                            &planet_pos,
                                            &planet_ball,
                                            0.0) {
                self.update_reset(window, true);
                return (self.attached_planet, self.height);
            }
        }

        for (planet_index, planet) in self.space.get_nearby_planets() {
            let planet_ball = Ball::new(planet.radius);
            let planet_pos = coll_pt(planet.pos);

            // Check if this is the closest planet
            let distance = query::distance(&na_ship_pos, &ship_ball, &planet_pos, &planet_ball);
            if distance < closest_planet_distance {
                closest_planet_distance = distance;
                closest_planet_idx = planet_index;
                self.closest_planet_coords = pt(planet.pos.x, planet.pos.y);
            }
            let collided =
                query::contact(&na_ship_pos, &ship_ball, &planet_pos, &planet_ball, -1.0);
            if let Some(_) = collided {
                // We are landing on a new planet
                self.attached_planet = planet_index;
                self.flying = false;
                self.height = planet.radius + (SHIP_SIZE / 2.0);
                self.rotation = direction_from_to(planet.pos, ship_pos);

                if planet.bouncy {
                    self.jumping = true;
                    self.exit_speed = JUMP_SPEED;
                } else {
                    self.jumping = false;
                    self.exit_speed = 0.0;
                }
            }
        }
        (closest_planet_idx, closest_planet_distance)
    }

    /// Handle use of the "attach" ability
    fn update_attach(&mut self,
                     closest_planet_idx: PlanetIndex,
                     closest_planet_distance: f64,
                     ship_pos: Point,
                     input: &GameInput) {
        if input.attach && closest_planet_idx != self.attached_planet {
            self.attached_planet = closest_planet_idx;
            self.exit_speed = 0.0;
            self.rotation = (ship_pos.y - self.closest_planet_coords.y)
                .atan2(ship_pos.x - self.closest_planet_coords.x);
            self.height = closest_planet_distance +
                          self.space.get_planet(closest_planet_idx).radius +
                          (SHIP_SIZE / 2.0);
        }

    }

    // Update the camera so that the ship stays in view with a margin around the screen.
    fn update_camera(&self,
                     view_size: Size,
                     on_planet: Option<Point>,
                     camera_pos: Point,
                     ship_pos: Point)
                     -> Point {
        match on_planet {
            Some(planet_center) => {
                pt(lerp(camera_pos.x,
                        planet_center.x - (view_size.width as f64 / 2.0),
                        0.03),
                   lerp(camera_pos.y,
                        planet_center.y - (view_size.height as f64 / 2.0),
                        0.03))
            }
            None => {
                let x_margin = (1.0 / 3.0) * view_size.width as f64;
                let y_margin = (1.0 / 3.0) * view_size.height as f64;
                let view_width_with_margin = view_size.width as f64 * (2.0 / 3.0);
                let view_height_with_margin = view_size.height as f64 * (2.0 / 3.0);

                // this seems like it's way more complicated than it should be
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
                // pt((camera_pos.x + x) / 2.0, (camera_pos.y + y) / 2.0)
                pt(lerp(camera_pos.x, x, 0.1), lerp(camera_pos.y, y, 0.1))


            }
        }
    }

    fn update_bugs(&mut self, mut window: &mut PistonWindow, ship_pos: Point, time_delta: f64) {
        let bball = Ball::new(BULLET_SIZE);
        let crawler_ball = Ball::new(CRAWLER_SIZE);
        for (area, crawler_idx) in self.space.get_nearby_bugs() {
            {
                let bug = self.space.get_bug_mut(area, crawler_idx);
                if bug.moves_right {
                    bug.rotation += CRAWLER_SPEED * time_delta;
                } else {
                    bug.rotation -= CRAWLER_SPEED * time_delta;
                }
            }
            let crawler_pos = {
                let crawler = self.space.get_bug(area, crawler_idx);
                let planet = self.space.get_planet(crawler.attached);
                let bug_pos =
                    rotated_position(planet.pos, crawler.rotation, planet.radius + CRAWLER_SIZE);
                coll_pt(bug_pos)
            };
            {
                let na_ship_pos = coll_pt(ship_pos);
                let ship_ball = Ball::new(SHIP_SIZE / 2.0);
                let uhoh =
                    query::contact(&crawler_pos, &crawler_ball, &na_ship_pos, &ship_ball, 0.0);
                if let Some(_) = uhoh {
                    self.update_reset(&mut window, false);
                    return;
                }
            }
            for bullet in self.bullets.iter() {
                let bpos = coll_pt(bullet.pos);
                let collided = query::contact(&crawler_pos, &crawler_ball, &bpos, &bball, 0.0);
                if let Some(_) = collided {
                    self.space.delete_bug(area, crawler_idx);
                }
            }
        }
    }
}

fn fill_circle(canvas: &mut im::ImageBuffer<im::Rgba<u8>, Vec<u8>>,
               color: im::Rgba<u8>,
               radius: i32,
               x0: i32,
               y0: i32) {
    let mut x = radius;
    let mut y = 0;
    let mut err = 0;
    while x >= y {
        // we draw horizontal lines
        for this_x in (x0 - y)..(x0 + y) {
            canvas.put_pixel(this_x as u32, (y0 + x) as u32, color);
        }
        for this_x in (x0 - x)..(x0 + x) {
            canvas.put_pixel(this_x as u32, (y0 + y) as u32, color);
        }
        for this_x in (x0 - y)..(x0 + y) {
            canvas.put_pixel(this_x as u32, (y0 - x) as u32, color);
        }
        for this_x in (x0 - x)..(x0 + x) {
            canvas.put_pixel(this_x as u32, (y0 - y) as u32, color);
        }
        y += 1;
        err += 1 * 2 * y;
        if 2 * (err - x) + 1 > 0 {
            x -= 1;
            err += 1 - 2 * x;
        }
    }
}

fn generate_minimap(window: &mut PistonWindow,
                    space: &Space,
                    (min, max): (Point, Point))
                    -> G2dTexture {
    let mut canvas: im::ImageBuffer<im::Rgba<u8>, Vec<u8>> =
        im::ImageBuffer::from_pixel(MINI_WIDTH as u32,
                                    MINI_HEIGHT as u32,
                                    im::Rgba([255, 255, 255, 8]));
    let planet_pixel = im::Rgba([0, 0, 255, 255]);
    let bouncy_pixel = im::Rgba([127, 127, 255, 255]);
    let magic_pixel = im::Rgba([255, 0, 0, 255]);
    canvas.put_pixel(100, 50, planet_pixel);

    fn render_planet(mut canvas: &mut im::ImageBuffer<im::Rgba<u8>, Vec<u8>>,
                     color: im::Rgba<u8>,
                     pos: Point,
                     radius: f64,
                     min: Point,
                     max: Point) {
        let (mini_x, mini_y) = shrink_to_bounds(MINI_WIDTH, MINI_HEIGHT, min, max, pos);
        let rad = radius / ((max.x - min.x) / MINI_WIDTH);
        fill_circle(&mut canvas, color, rad as i32, mini_x as i32, mini_y as i32);
    }

    for planet in space.get_all_planets() {
        let pixel = if planet.bouncy { bouncy_pixel } else { planet_pixel };
        render_planet(&mut canvas, pixel, planet.pos, planet.radius, min, max);
    }
    render_planet(&mut canvas,
                  magic_pixel,
                  space.get_magic_planet(),
                  MAGIC_PLANET_SIZE,
                  min,
                  max);
    Texture::from_image(&mut window.factory, &canvas, &TextureSettings::new()).unwrap()
}

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

    // Create a new game and run it.
    let space = Space::new();
    let attached_planet_idx = space.get_first_planet();
    let space_bounds = space.get_space_bounds();
    let mut app = App {
        score: 0,
        debug: false,
        camera_pos: pt(-0.0, -0.0),
        flying: false,
        jumping: false,
        fire_cooldown: 0.0,
        exit_speed: 0.0,
        height: space.get_planet(attached_planet_idx).radius,
        rotation: 0.0,
        attached_planet: attached_planet_idx,
        closest_planet_coords: space.get_planet(attached_planet_idx).pos,
        bullets: vec![],
        space_bounds: space_bounds,
        minimap: generate_minimap(&mut window, &space, space_bounds),
        space: space,
    };

    let mut input = GameInput::new();
    let mut cursor: Option<[f64; 2]> = None;
    let mut shooting = false;

    let mut fps_counter = fps_counter::FPSCounter::new();
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
            app.update(&u, &mut window, &input);
            if input.attach {
                input.attach = false;
            }
            if input.toggle_debug {
                input.toggle_debug = false;
            }
        }
        app.render(&mut window, &e, &mut glyphs, &mut fps_counter);
    }
}
