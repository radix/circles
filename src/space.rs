extern crate rand;

use std::collections::HashMap;
use std::f64::consts::PI;
use std::sync::atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT};

use self::rand::distributions::{IndependentSample, Range};
use calc::*;

const AREA_WIDTH: f64 = 2560.0;
const AREA_HEIGHT: f64 = 2560.0;


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
    pub moves_right: bool,
}

pub type Area = (i32, i32);

/// Space is responsible for holding all the planets in the universe, generating planets when the
/// ship moves through space, and also giving a view of nearby planets. It is responsible for
/// holding the position of the ship (`current_point`) to give a safe way to see nearby planets.
#[derive(Debug)]
pub struct Space {
    // Keep this private!
    // this uses a HashMap for CrawlerBugs so that they're easier to delete
    areas: HashMap<Area, (Vec<Planet>, HashMap<usize, CrawlerBug>)>,
    current_point: Point,
    magic_planet: Point,
}

impl Space {
    pub fn new() -> Self {
        let mut sp = Space {
            areas: HashMap::new(),
            current_point: pt(0.0, 0.0),
            magic_planet: pt(0.0, 0.0),
        };
        sp.generate_level();
        sp
    }

    pub fn focus(&mut self, p: Point) {
        self.current_point = p;
    }

    pub fn get_focus(&self) -> Point {
        self.current_point
    }

    pub fn get_magic_planet(&self) -> Point {
        self.magic_planet
    }

    /// Return all nearby planets
    pub fn get_nearby_planets(&self) -> Vec<(PlanetIndex, &Planet)> {
        self.get_nearby_areas()
            .iter()
            .flat_map::<Vec<(PlanetIndex, &Planet)>, _>(|area| {
                if let Some(&(ref planets, _)) = self.areas.get(&area) {
                    planets.iter()
                        .enumerate()
                        .map(|(i, p)| {
                            (PlanetIndex {
                                 area: area.clone(),
                                 idx: i,
                             },
                             p)
                        })
                        .collect()
                } else {
                    vec![]
                }

            })
            .collect()
    }

    // Return indices of nearby CrawlerBugs.
    pub fn get_nearby_bugs(&self) -> Vec<(Area, usize)> {
        self.get_nearby_areas()
            .iter()
            .flat_map::<Vec<(Area, usize)>, _>(|area| {
                if let Some(&(_, ref bugs)) = self.areas.get(&area) {
                    bugs.keys().map(|i| (area.clone(), i.clone())).collect()
                } else {
                    vec![]
                }
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

    fn area_for_point(p: Point) -> Area {
        let x = p.x / AREA_WIDTH;
        let y = p.y / AREA_HEIGHT;
        (x.floor() as i32, y.floor() as i32)
    }

    pub fn get_central_area(&self) -> Area {
        Space::area_for_point(self.current_point)
    }

    pub fn get_first_planet(&self) -> PlanetIndex {
        PlanetIndex {
            area: (0, 0),
            idx: 0,
        }
    }

    /// Generate planets around the current center point
    fn generate_level(&mut self) {
        const MIN_PLANET_DISTANCE: f64 = 150.0;
        const MAX_PLANET_DISTANCE: f64 = 400.0;
        const MIN_PLANET_SIZE: f64 = 35.0;
        const MAX_PLANET_SIZE: f64 = 100.0;
        const PATH_VARIANCE: f64 = 0.1; // applied to radians
        const CRAWLER_PERCENTAGE: f64 = 0.5;


        // use an absolute bug count to index bugs so that we can safely delete them even while
        // looping over them.
        // I use Atomic only so I can avoid "unsafe" blocks which would be needed for static mut
        static BUG_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

        let range_circle = Range::new(-2.0 * PI, 2.0 * PI);
        let range_distance = Range::new(MIN_PLANET_DISTANCE, MAX_PLANET_DISTANCE);
        let range_radius = Range::new(MIN_PLANET_SIZE, MAX_PLANET_SIZE);
        let range_direction = Range::new(-PI * PATH_VARIANCE, PI * PATH_VARIANCE);
        let range_crawler_exists = Range::new(0.0, 1.0);
        let range_bool = Range::new(0, 2);
        let mut rng = rand::thread_rng();

        self.areas.insert((0, 0),
                          (vec![Planet {
                                    radius: 50.0,
                                    pos: pt(0.0, 0.0),
                                }],
                           HashMap::new()));
        let mut prev_pos = pt(0.0, 0.0);
        let mut prev_rot = range_circle.ind_sample(&mut rng);

        for _ in 0..25 {
            let radius = range_radius.ind_sample(&mut rng);
            let distance = range_distance.ind_sample(&mut rng);
            let direction = range_direction.ind_sample(&mut rng);
            let direction = direction + prev_rot;
            let pos = rotated_position(prev_pos, direction, distance);
            let planet = Planet {
                pos: pos,
                radius: radius,
            };
            let area = Self::area_for_point(pos);
            self.areas.entry(area).or_insert((vec![], HashMap::new()));
            if let Some(area_content) = self.areas.get_mut(&area) {
                area_content.0.push(planet);
            }
            prev_pos = pos;
            prev_rot = direction;
            // and the bug
            if range_crawler_exists.ind_sample(&mut rng) > CRAWLER_PERCENTAGE {
                let rot = range_circle.ind_sample(&mut rng);
                let planet_num = self.areas[&area].0.len() - 1;
                self.areas
                    .get_mut(&area)
                    .unwrap()
                    .1
                    .insert(BUG_COUNT.fetch_add(1, Ordering::SeqCst),
                            CrawlerBug {
                                moves_right: range_bool.ind_sample(&mut rng) == 1,
                                rotation: rot,
                                attached: PlanetIndex {
                                    area: area,
                                    idx: planet_num,
                                },
                            });
            }
        }

        self.magic_planet = rotated_position(prev_pos, prev_rot, 400.0);
    }
}
