extern crate rand;

use std::collections::HashMap;
use std::fmt;

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
    area: (i32, i32),
    idx: usize,
}

impl PlanetIndex {
    pub fn get_area(&self) -> (i32, i32) {
        self.area
    }
}

#[derive(Debug)]
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
    areas: HashMap<Area, (Vec<Planet>, Vec<CrawlerBug>)>,
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

    pub fn get_nearby_bugs(&self) -> Vec<&CrawlerBug> {
        self.get_nearby_areas()
            .iter()
            .flat_map(|area| {
                &self.areas
                    .get(&area)
                    .expect(&format!("Uninitialized BUG area {:?} when in area {:?}",
                                     area,
                                     self.get_central_area()))
                    .1
            })
            .collect()
    }

    pub fn get_nearby_bugs_mut(&mut self) -> Vec<&mut CrawlerBug> {
        let nearby_areas = self.get_nearby_areas();
        let mut buggies = vec![];
        // okay, so long story.

        // Context: This function wants to return multiple mutable references in a Vec -- meaning,
        // they will live at the same time. Normally, rust will not allow multiple mutable
        // references into the same value, even if they are "disjoint" -- that is, non-overlapping
        // in memory. That means that if I try to call HashMap::get_mut a bunch of times and save
        // away all of those mutable references, rustc will barf because it can't prove I didn't
        // pass the same key to get_mut multiple times (meaning the same &mut value would be
        // returned multiple times, which would actually be unsafe).

        // However, HashMap and other built-in datatypes have a trick up their sleeves: .iter_mut()
        // apparently does some unsafe magic to hand out mutable references that can live at the
        // same time. The reasoning why this is safe is simple: since it's iterating over all the
        // values only once, it can be sure that it's only handing out one &mut per value -- that
        // is, all returned references are disjoint.

        // Of course, this means that we have to walk the entire hashmap, when we really just need
        // access to a few keys, so this is not a scalable solution.
        for (k, &mut (_, ref mut v)) in self.areas.iter_mut() {
            if nearby_areas.contains(k) {
                buggies.extend(v);
            }
        }
        buggies
    }

    /// Look up a specific planet. This is safe only when using a PlanetIndex returned from *this
    /// instance's* get_nearby_planets method
    pub fn get_planet(&self, idx: PlanetIndex) -> &Planet {
        &self.areas[&idx.area].0[idx.idx]
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
        let mut rng = rand::thread_rng();
        let bugs = planets.iter()
            .filter_map(|&(pidx, _)| {
                // half the planets will have bugaroos.
                let rando = range_bool.ind_sample(&mut rng);
                if rando == 1 {
                    Some(CrawlerBug {
                        attached: pidx,
                        rotation: 0.0,
                    })
                } else {
                    None
                }
            })
            .collect();
        bugs
    }
}
