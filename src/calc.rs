extern crate nalgebra as na;
use std::fmt;

use na::{Isometry2, Vector2};


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
        write!(f, "{:.0},{:.0}", self.x, self.y)
    }
}

pub fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// Get the direction (in Radians) from one point to another.
pub fn direction_from_to(from: Point, to: Point) -> f64 {
    (to.y - from.y).atan2(to.x - from.x)
}

pub fn rotated_position(origin: Point, rotation: f64, height: f64) -> Point {
    pt(origin.x + (rotation.cos() * height),
       origin.y + (rotation.sin() * height))
}

pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + t * (b - a)
}

pub fn coll_pt(point: Point) -> Isometry2<f64> {
    Isometry2::new(Vector2::new(point.x, point.y), na::zero())
}

pub fn shrink_to_bounds(mini_width: f64,
                        mini_height: f64,
                        min: Point,
                        max: Point,
                        point: Point)
                        -> (u32, u32) {
    let width = max.x - min.x;
    let height = max.y - min.y;
    let percent_x = (point.x - min.x) / width;
    let percent_y = (point.y - min.y) / height;
    let mini_x = percent_x * mini_width;
    let mini_y = percent_y * mini_height;
    (mini_x.floor() as u32, mini_y.floor() as u32)
}
