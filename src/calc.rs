use std::fmt;


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
