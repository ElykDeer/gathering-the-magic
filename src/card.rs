use opencv::{
    core::{Mat, Point, Scalar, Vector},
    imgproc::{contour_area, line, LineTypes},
};
use std::{collections::HashMap, time::SystemTime};

pub(crate) fn distance_formula(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    (((x2 - x1) as f64).powi(2) + ((y2 - y1) as f64).powi(2)).sqrt()
}

pub struct Card {
    pub rect: [[i32; 2]; 4],
    pub last_seen: SystemTime,
    pub alive: bool,
    pub x: i32,
    pub y: i32,
    pub radius: f64,
    pub area: f64,
    pub contour: Vector<Point>,
    pub hashes: HashMap<u64, u64>, // hash of the oracle, cumulative distance
    pub id: String,
}

impl Card {
    pub fn new(rect: Vec<[i32; 2]>, contour: Vector<Point>) -> Self {
        let x = rect.iter().map(|[x, _]| x).sum::<i32>() / rect.len() as i32;
        let y = rect.iter().map(|[_, y]| y).sum::<i32>() / rect.len() as i32;

        let area = contour_area(
            &rect
                .iter()
                .map(|p| Point::new(p[0], p[1]))
                .collect::<Vector<Point>>(),
            false,
        )
        .unwrap()
        .abs();
        let radius = (area / std::f64::consts::PI).sqrt();

        Self {
            rect: rect.try_into().unwrap(),
            last_seen: SystemTime::now(),
            alive: true,
            x,
            y,
            radius,
            area,
            contour,
            hashes: HashMap::new(),
            id: "".to_string(),
        }
    }

    pub fn update(&mut self, new_card: Card) {
        if self.alive && distance_formula(new_card.x, new_card.y, self.x, self.y) < self.radius {
            self.rect = new_card.rect;
            self.last_seen = new_card.last_seen;
            self.x = new_card.x;
            self.y = new_card.y;
            self.contour = new_card.contour.clone();
        } else {
            *self = new_card;
        }
    }

    // Set myself as stale if we haven't seen anything a while
    pub fn prune(&mut self) {
        if self.alive && self.last_seen.elapsed().unwrap().as_secs_f64() > 1.0 {
            self.alive = false;
        }
    }

    pub fn draw(&self, frame: &mut Mat) {
        if self.alive {
            // Boxes
            for i in 0..4 {
                let a = self.rect[i];
                let b = self.rect[(i + 1) % 4];
                line(
                    frame,
                    Point::new(a[0], a[1]),
                    Point::new(b[0], b[1]),
                    Scalar::new(0xFE as f64, 0xFF as f64, 0x01 as f64, 0.0),
                    5,
                    LineTypes::LINE_4 as i32,
                    0,
                )
                .unwrap();
            }
        }
    }
}

impl Default for Card {
    fn default() -> Self {
        Self {
            rect: <[_; 4]>::default(),
            last_seen: SystemTime::now(),
            alive: false,
            x: 0,
            y: 0,
            radius: 0.0,
            area: 0.0,
            contour: Vector::default(),
            hashes: HashMap::new(),
            id: "".to_string(),
        }
    }
}
