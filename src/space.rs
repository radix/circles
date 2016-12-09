extern crate rand;

use std::collections::HashMap;
use std::fmt;
use std::f64::consts::PI;

use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

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

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:.0}/{:.0}", self.x, self.y)
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanetIndex {
    // Keep this private, so users can't construct PlanetIndexes. This marginally improves safety,
    // since Space::get_planet is unsafe if passed PlanetIndexes that weren't returned from
    // Space::get_nearby_planets. However, it's only marginal since the user could have multiple
    // Space instances, and cause panics by passing one space's PlanetIndex objects to a different
    // space. Need dependent types.
    area: Area,
    idx: usize,
}

impl PlanetIndex {
    pub fn get_area(&self) -> Area {
        self.area
    }
}

#[derive(Debug, PartialEq)]
pub struct CrawlerBug {
    pub attached: PlanetIndex,
    pub rotation: f64,
}

type Area = (i32, i32);

/// Space is responsible for holding all the planets in the universe, generating planets when the
/// ship moves through space, and also giving a view of nearby planets. It is responsible for
/// holding the position of the ship (`current_point`) to give a safe way to see nearby planets.
#[derive(Debug)]
pub struct Space {
    // Keep this private!
    areas: HashMap<Area, (Vec<Planet>, HashMap<usize, CrawlerBug>)>,
    current_point: Point,
}

impl Space {
    pub fn new() -> Space {
        let mut sp = Space {
            areas: HashMap::new(),
            current_point: pt(0.0, 0.0),
        };
        sp.realize();
        sp
    }

    pub fn focus(&mut self, p: Point) {
        self.current_point = p;
        self.realize();
    }

    pub fn get_focus(&self) -> Point {
        return self.current_point;
    }

    /// Return all nearby planets and bugs.
    pub fn get_nearby_planets(&self) -> Vec<(PlanetIndex, &Planet)> {
        self.get_nearby_areas()
            .iter()
            .flat_map(|area| {
                let ref planets = self.areas
                    .get(&area)
                    .expect(&format!("Uninitialized PLANET area {:?} when in area {:?}",
                                     area,
                                     self.get_central_area()))
                    .0;
                let planets: Vec<(PlanetIndex, &Planet)> = planets.iter()
                    .enumerate()
                    .map(|(i, p)| {
                        (PlanetIndex {
                             area: area.clone(),
                             idx: i,
                         },
                         p)
                    })
                    .collect();
                planets
            })
            .collect()
    }

    pub fn get_nearby_bugs(&self) -> Vec<(Area, usize)> {
        self.get_nearby_areas()
            .iter()
            .flat_map(|area| {
                let x: Vec<(Area, usize)> = self.areas
                    .get(&area)
                    .unwrap()
                    .1
                    .keys()
                    .map(|i| (area.clone(), i.clone()))
                    .collect();
                x
            })
            .collect()
    }

    /// Get a bug. Only safe when using Area and usize as returned from get_nearby_bugs.
    pub fn get_bug(&self, area: Area, idx: usize) -> &CrawlerBug {
        self.areas[&area].1.get(&idx).unwrap()
    }
    /// Get a mutable bug. Only safe when using Area and usize as returned from get_nearby_bugs.
    pub fn get_bug_mut(&mut self, area: Area, idx: usize) -> &mut CrawlerBug {
        self.areas.get_mut(&area).unwrap().1.get_mut(&idx).unwrap()
    }
    /// Delete a bug. Only safe when using Area and usize as returned from get_nearby_bugs.
    pub fn delete_bug(&mut self, area: Area, idx: usize) {
        let ref mut bugs = self.areas.get_mut(&area).unwrap().1;
        bugs.remove(&idx);
    }

    /// Look up a specific planet. This is safe only when using a PlanetIndex returned from *this
    /// instance's* get_nearby_planets method
    pub fn get_planet(&self, idx: PlanetIndex) -> &Planet {
        &self.areas[&idx.area].0[idx.idx]
    }

    fn get_nearby_areas(&self) -> Vec<Area> {
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

    fn get_area(p: Point) -> Area {
        let x = p.x / AREA_WIDTH;
        let y = p.y / AREA_HEIGHT;
        (x.floor() as i32, y.floor() as i32)
    }

    pub fn get_central_area(&self) -> Area {
        Space::get_area(self.current_point)
    }

    /// Generate planets around the current center point
    fn realize(&mut self) {
        // use an absolute bug count to index bugs so that we can safely delete them even while
        // looping over them.
        // I only use Atomic so I can avoid "unsafe" blocks
        static BUG_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;
        for area in self.get_nearby_areas() {
            if !self.areas.contains_key(&area) {
                let mut planets = Space::gen_planets();
                for planet in planets.iter_mut() {
                    planet.pos.x += area.0 as f64 * AREA_WIDTH;
                    planet.pos.y += area.1 as f64 * AREA_HEIGHT;
                }
                let bugs = {
                    let planets_with_indices = planets.iter()
                        .enumerate()
                        .map(|(i, &ref p)| {
                            (PlanetIndex {
                                 area: area,
                                 idx: i,
                             },
                             p)
                        })
                        .collect();
                    Space::gen_bugs(&planets_with_indices)
                        .into_iter()
                        .map(|b| (BUG_COUNT.fetch_add(1, Ordering::SeqCst), b))
                        .collect()
                };

                self.areas.insert(area, (planets, bugs));
            }
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

    fn gen_bugs(planets: &Vec<(PlanetIndex, &Planet)>) -> Vec<CrawlerBug> {
        let range_bool = Range::new(0, 2);
        let range_rot = Range::new(-2.0 * PI, 2.0 * PI);
        let mut rng = rand::thread_rng();
        let bugs = planets.iter()
            .filter_map(|&(pidx, _)| {
                // half the planets will have bugaroos.
                let rando = range_bool.ind_sample(&mut rng);
                if rando == 1 {
                    Some(CrawlerBug {
                        attached: pidx,
                        rotation: range_rot.ind_sample(&mut rng),
                    })
                } else {
                    None
                }
            })
            .collect();
        bugs
    }
}
