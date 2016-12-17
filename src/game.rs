use std::f64::consts::PI;
use piston_window::*;
use ncollide::query;
use ncollide::shape::Ball;

use im;

use space::*;
use calc::*;

pub const SHIP_SIZE: f64 = 50.0;
pub const CRAWLER_SIZE: f64 = 25.0;
pub const CRAWLER_SPEED: f64 = 2.0;
pub const SPEED: f64 = 5.0;
pub const FLY_SPEED: f64 = 3.0;
pub const JUMP_SPEED: f64 = 10.0;
pub const AIR_CONTROL_MOD: f64 = 0.40;
pub const BULLET_SPEED: f64 = 1000.0;
pub const BULLET_SIZE: f64 = 5.0;
pub const ACCELERATION: f64 = 5.0;
pub const FIRE_COOLDOWN: f64 = 0.1;
pub const GRAVITY: f64 = 20.0;
pub const MINI_SIZE: f64 = 200.0;

pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const DARKRED: [f32; 4] = [0.5, 0.0, 0.0, 1.0];
pub const LIGHTBLUE: [f32; 4] = [0.5, 0.5, 1.0, 1.0];
pub const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

pub struct GameInput {
    pub toggle_debug: bool,
    pub left: bool,
    pub right: bool,
    pub down: bool,
    pub up: bool,
    pub jump: bool,
    pub shoot_target: Option<Point>,
    pub attach: bool,
}

impl GameInput {
    pub fn new() -> GameInput {
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

pub struct App {
    // meta-state? or something
    pub debug: bool,
    // rendering state
    // glyphs: Glyphs
    pub minimap: G2dTexture,
    pub space_bounds: (Point, Point), // min and max
    // gameplay state
    pub space: Space,
    pub score: i8,
    pub rotation: f64, // ship rotation / position along the orbit
    pub flying: bool,
    pub jumping: bool,
    pub exit_speed: f64,
    pub height: f64,
    pub attached_planet: PlanetIndex,
    pub closest_planet_coords: Point, // redundant data, optimization
    // NES-style would be to make this a [(f64, f64); 3], so only three bullets can exist at once
    pub bullets: Vec<Bullet>,
    pub fire_cooldown: f64,
    pub camera_pos: Point,
}

impl App {
    pub fn new(mut window: &mut PistonWindow) -> Self {
        // Create a new game and run it.
        let space = Space::new();
        let attached_planet_idx = space.get_first_planet();
        let space_bounds = space.get_space_bounds();
        App {
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
        }
    }

    pub fn update(&mut self, args: &UpdateArgs, window: &mut PistonWindow, input: &GameInput) {
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
    let mini_size = MINI_SIZE as i32;

    let mut draw_line = |x_center: i32, x_offset: i32, y: i32| {
        for this_x in (x_center - x_offset)..(x_center + x_offset) {
            if this_x >= 0 && this_x < mini_size && y >= 0 && y < mini_size {
                canvas.put_pixel(this_x as u32, y as u32, color);
            }
        }
    };
    while x >= y {
        // we draw horizontal lines
        draw_line(x0, y, y0 + x);
        draw_line(x0, x, y0 + y);
        draw_line(x0, y, y0 - x);
        draw_line(x0, x, y0 - y);
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
    let planet_pixel = im::Rgba([0, 0, 255, 255]);
    let bouncy_pixel = im::Rgba([127, 127, 255, 255]);
    let magic_pixel = im::Rgba([255, 0, 0, 255]);


    let mut canvas: im::ImageBuffer<im::Rgba<u8>, Vec<u8>> =
        im::ImageBuffer::from_pixel(MINI_SIZE as u32,
                                    MINI_SIZE as u32,
                                    im::Rgba([255, 255, 255, 8]));
    {
        let mut render_planet = |color, pos, radius| {
            let (mini_x, mini_y) = shrink_to_bounds(MINI_SIZE, MINI_SIZE, min, max, pos);
            let rad = radius / ((max.x - min.x) / MINI_SIZE);
            fill_circle(&mut canvas, color, rad as i32, mini_x as i32, mini_y as i32);
        };

        for planet in space.get_all_planets() {
            let pixel = if planet.bouncy { bouncy_pixel } else { planet_pixel };
            render_planet(pixel, planet.pos, planet.radius);
        }
        render_planet(magic_pixel, space.get_magic_planet(), MAGIC_PLANET_SIZE);
    }
    Texture::from_image(&mut window.factory, &canvas, &TextureSettings::new()).unwrap()
}
