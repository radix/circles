extern crate rand;

use std::collections::HashMap;

use self::rand::distributions::{IndependentSample, Range};


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
    // Keep this private!
    area: (i32, i32),
    idx: usize,
}

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
            current_point: Point::new(0.0, 0.0),
        };
        sp.realize();
        sp
    }

    pub fn focus(&mut self, p: Point) {
        self.current_point = p;
        self.realize();
    }

    /// Return all nearby planets, but without generating any. This means that if you call this
    /// with a point that hasn't been realize()d yet, it may panic.
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
                       Space::get_area(self.current_point));
            }
        }
        planets
    }

    /// This is safe only when using a PlanetIndex returned from *this instance's* get_nearby_planets function
    pub fn get_planet(&self, idx: PlanetIndex) -> &Planet {
        &self.planets[&idx.area][idx.idx]
    }

    fn get_nearby_areas(&self) -> Vec<(i32, i32)> {
        let area = Space::get_area(self.current_point);
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
        (p.x.floor() as i32 / 1024, p.y.floor() as i32 / 768)
    }

    /// Generate planets around the current center point
    fn realize(&mut self) {
        for area in self.get_nearby_areas() {
            self.planets.entry(area).or_insert_with(|| Space::gen_planets(area));
        }
    }

    /// Generate some planets for a given area. Each planet will have an appropriate x/y offset based
    /// on the area given.
    fn gen_planets(area: (i32, i32)) -> Vec<Planet> {
        println!("Generating planets for {:?}", area);
        let x_offset = 1024.0 * (area.0 as f64);
        let y_offset = 768.0 * (area.1 as f64);
        let range_width = Range::new(x_offset + 0.0, x_offset + 1024.0);
        let range_height = Range::new(y_offset + 0.0, y_offset + 768.0);
        let range_radius = Range::new(35.0, 100.0);
        let mut rng = rand::thread_rng();
        let range_num_planets = Range::new(1, 5);
        let num_planets = range_num_planets.ind_sample(&mut rng);
        (0..num_planets)
            .map(|_| {
                let x = range_width.ind_sample(&mut rng);
                let y = range_height.ind_sample(&mut rng);
                let radius = range_radius.ind_sample(&mut rng);
                Planet {
                    pos: Point::new(x, y),
                    radius: radius,
                }
            })
            .collect()
    }
}
