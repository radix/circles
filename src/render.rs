use std::f64::consts::PI;

use piston_window::{G2d, Glyphs, Window, PistonWindow, Event, Context, Rectangle, Size,
                    Transformed, clear, rectangle, ellipse, line, image, text};
use fps_counter;

use game::{App, BULLET_SIZE, MINI_SIZE, SHIP_SIZE, CRAWLER_SIZE};
use calc::{Point, shrink_to_bounds, rotated_position, direction_from_to};
use space::{Area, Planet, PlanetIndex, MAGIC_PLANET_SIZE};


pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const DARKRED: [f32; 4] = [0.5, 0.0, 0.0, 1.0];
pub const LIGHTBLUE: [f32; 4] = [0.5, 0.5, 1.0, 1.0];
pub const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];


type Transform = [[f64; 3]; 2];

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

    pub fn render(&self,
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
            let (mini_x, mini_y) = shrink_to_bounds(MINI_SIZE,
                                                    MINI_SIZE,
                                                    self.space_bounds.0,
                                                    self.space_bounds.1,
                                                    self.space.get_focus());

            let size = SHIP_SIZE / ((self.space_bounds.1.x - self.space_bounds.0.x) / MINI_SIZE);

            rectangle(RED,
                      rectangle::square(0.0, 0.0, size),
                      trans.trans(mini_x as f64, mini_y as f64).rot_rad(self.rotation),
                      g);
        }
        {
            let r = rectangle::rectangle_by_corners(0.0, 0.0, MINI_SIZE, MINI_SIZE);
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
}

/// Check if a circle is in the current viewport.
fn circle_in_view(point: Point, radius: f64, camera: Point, view_size: Size) -> bool {
    point.x + radius > camera.x && point.x - radius < camera.x + view_size.width as f64 ||
    point.y + radius > camera.y && point.y - radius < camera.y + view_size.height as f64
}
