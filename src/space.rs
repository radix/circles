extern crate rand;

use std::collections::HashMap;

use self::rand::distributions::{IndependentSample, Range};

const AREA_WIDTH: f64 = 2560.0;
const AREA_HEIGHT: f64 = 2560.0;


#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Point {
        Point { x: x, y: y }
    }
}

pub fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

#[derive(Debug)]
pub struct Planet {
    pub pos: Point,
    pub radius: f64,
}

#[derive(Debug)]
pub struct Bullet {
    pub pos: Point,
    pub dir: f64, // radians
    pub speed: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct PlanetIndex {
    // Keep this private, so users can't construct PlanetIndexes. This marginally improves safety,
    // since Space::get_planet is unsafe if passed PlanetIndexes that weren't returned from
    // Space::get_nearby_planets. However, it's only marginal since the user could have multiple
    // Space instances, and cause panics by passing one space's PlanetIndex objects to a different
    // space. Need dependent types.
    area: (i32, i32),
    idx: usize,
}

// impl PlanetIndex {
//     pub fn get_area(&self) -> (i32, i32) {
//         self.area
//     }
//     pub fn get_index(&self) -> usize {
//         self.idx
//     }
// }


/// Space is responsible for holding all the planets in the universe, generating planets when the
/// ship moves through space, and also giving a view of nearby planets. It is responsible for
/// holding the position of the ship (`current_point`) to give a safe way to see nearby planets.
#[derive(Debug)]
pub struct Space {
    // Keep this private!
    planets: HashMap<(i32, i32), Vec<Planet>>,
    current_point: Point,
}

impl Space {
    pub fn new() -> Space {
        let mut sp = Space {
            planets: HashMap::new(),
            current_point: pt(0.0, 0.0),
        };
        sp.realize();
        sp
    }

    pub fn focus(&mut self, p: Point) {
        let old_area = self.get_central_area();
        self.current_point = p;
        self.realize();
        if self.get_central_area() != old_area {
            println!("Moving area: {:?} ship is at {:?}",
                     self.get_central_area(),
                     p);
        }
    }

    pub fn get_focus(&self) -> Point {
        return self.current_point;
    }

    /// Return all nearby planets.
    pub fn get_nearby_planets(&self) -> Vec<(PlanetIndex, &Planet)> {
        let mut planets = vec![];
        for area in self.get_nearby_areas() {
            if let Some(ps) = self.planets.get(&area) {
                for (i, p) in ps.iter().enumerate() {
                    planets.push((PlanetIndex {
                                      area: area,
                                      idx: i,
                                  },
                                  p));
                }
            } else {
                panic!("Uninitialized area {:?} when in area {:?}",
                       area,
                       self.get_central_area());
            }
        }
        planets
    }

    /// Look up a specific planet. This is safe only when using a PlanetIndex returned from *this
    /// instance's* get_nearby_planets method
    pub fn get_planet(&self, idx: PlanetIndex) -> &Planet {
        &self.planets[&idx.area][idx.idx]
    }

    fn get_nearby_areas(&self) -> Vec<(i32, i32)> {
        let area = self.get_central_area();
        let (x, y) = (area.0, area.1);
        vec![(x - 1, y - 1),
             (x - 1, y + 0),
             (x - 1, y + 1),
             (x + 0, y - 1),
             (x + 0, y + 0),
             (x + 0, y + 1),
             (x + 1, y - 1),
             (x + 1, y + 0),
             (x + 1, y + 1)]
    }

    fn get_area(p: Point) -> (i32, i32) {
        let x = p.x / AREA_WIDTH;
        let y = p.y / AREA_HEIGHT;
        (x.floor() as i32, y.floor() as i32)
    }

    pub fn get_central_area(&self) -> (i32, i32) {
        Space::get_area(self.current_point)
    }

    /// Generate planets around the current center point
    fn realize(&mut self) {
        for area in self.get_nearby_areas() {
            self.planets.entry(area).or_insert_with(|| {
                let mut planets = Space::gen_planets();
                println!("Area: {:?}", area);
                for planet in planets.iter_mut() {
                    println!("Planet: {:?}", planet);
                    planet.pos.x += area.0 as f64 * AREA_WIDTH;
                    planet.pos.y += area.1 as f64 * AREA_HEIGHT;
                    println!("Post-processed: {:?}", planet);
                }
                planets
            });
        }
    }

    /// Generate some planets in an area
    fn gen_planets() -> Vec<Planet> {
        let range_width = Range::new(0.0, AREA_WIDTH);
        let range_height = Range::new(0.0, AREA_HEIGHT);
        let range_radius = Range::new(35.0, 100.0);
        let mut rng = rand::thread_rng();
        let range_num_planets = Range::new(8, 16);
        let num_planets = range_num_planets.ind_sample(&mut rng);
        (0..num_planets)
            .map(|_| {
                let x = range_width.ind_sample(&mut rng);
                let y = range_height.ind_sample(&mut rng);
                let radius = range_radius.ind_sample(&mut rng);
                Planet {
                    pos: pt(x, y),
                    radius: radius,
                }
            })
            .collect()
    }
}
