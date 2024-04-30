use anyhow::Result;
use opencv::{
    core::{Mat, Point, Point2f, Scalar, Vector},
    imgproc::{contour_area, line, min_area_rect, LineTypes},
};
use std::{collections::HashMap, time::SystemTime};

fn distance_formula(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    (((x2 - x1) as f64).powi(2) + ((y2 - y1) as f64).powi(2)).sqrt()
}

pub struct Card {
    pub rect: [[i32; 2]; 4],
    pub last_seen: SystemTime,
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
            x,
            y,
            radius,
            area,
            contour,
            hashes: HashMap::new(),
            id: "".to_string(),
        }
    }
}

pub(crate) fn find_cards(contours: &Vector<Vector<Point>>, frame: &mut Mat) -> Result<Vec<Card>> {
    let colors = vec![
        Scalar::new(0x00 as f64, 0xFF as f64, 0x00 as f64, 0.0),
        Scalar::new(0xFF as f64, 0x00 as f64, 0x00 as f64, 0.0),
        Scalar::new(0x00 as f64, 0x00 as f64, 0xFF as f64, 0.0),
        Scalar::new(0xFE as f64, 0xFF as f64, 0x01 as f64, 0.0),
        Scalar::new(0xFE as f64, 0xA6 as f64, 0xFF as f64, 0.0),
        Scalar::new(0x66 as f64, 0xDB as f64, 0xFF as f64, 0.0),
        Scalar::new(0x01 as f64, 0x64 as f64, 0x00 as f64, 0.0),
        Scalar::new(0x67 as f64, 0x00 as f64, 0x01 as f64, 0.0),
        Scalar::new(0x3A as f64, 0x00 as f64, 0x95 as f64, 0.0),
        Scalar::new(0xB5 as f64, 0x7D as f64, 0x00 as f64, 0.0),
        Scalar::new(0xF6 as f64, 0x00 as f64, 0xFF as f64, 0.0),
        Scalar::new(0xE8 as f64, 0xEE as f64, 0xFF as f64, 0.0),
        Scalar::new(0x00 as f64, 0x4D as f64, 0x77 as f64, 0.0),
        Scalar::new(0x92 as f64, 0xFB as f64, 0x90 as f64, 0.0),
        Scalar::new(0xFF as f64, 0x76 as f64, 0x00 as f64, 0.0),
        Scalar::new(0x00 as f64, 0xFF as f64, 0xD5 as f64, 0.0),
        Scalar::new(0x7E as f64, 0x93 as f64, 0xFF as f64, 0.0),
        Scalar::new(0x6C as f64, 0x82 as f64, 0x6A as f64, 0.0),
        Scalar::new(0x9D as f64, 0x02 as f64, 0xFF as f64, 0.0),
        Scalar::new(0x00 as f64, 0x89 as f64, 0xFE as f64, 0.0),
        Scalar::new(0x82 as f64, 0x47 as f64, 0x7A as f64, 0.0),
        Scalar::new(0xD2 as f64, 0x2D as f64, 0x7E as f64, 0.0),
        Scalar::new(0x00 as f64, 0xA9 as f64, 0x85 as f64, 0.0),
        Scalar::new(0x56 as f64, 0x00 as f64, 0xFF as f64, 0.0),
        Scalar::new(0x00 as f64, 0x24 as f64, 0xA4 as f64, 0.0),
        Scalar::new(0x7E as f64, 0xAE as f64, 0x00 as f64, 0.0),
        Scalar::new(0x3B as f64, 0x3D as f64, 0x68 as f64, 0.0),
        Scalar::new(0xFF as f64, 0xC6 as f64, 0xBD as f64, 0.0),
        Scalar::new(0x00 as f64, 0x34 as f64, 0x26 as f64, 0.0),
        Scalar::new(0x93 as f64, 0xD3 as f64, 0xBD as f64, 0.0),
        Scalar::new(0x17 as f64, 0xB9 as f64, 0x00 as f64, 0.0),
        Scalar::new(0x8E as f64, 0x00 as f64, 0x9E as f64, 0.0),
        Scalar::new(0x44 as f64, 0x15 as f64, 0x00 as f64, 0.0),
        Scalar::new(0x9F as f64, 0x8C as f64, 0xC2 as f64, 0.0),
        Scalar::new(0xA3 as f64, 0x74 as f64, 0xFF as f64, 0.0),
        Scalar::new(0xFF as f64, 0xD0 as f64, 0x01 as f64, 0.0),
        Scalar::new(0x54 as f64, 0x47 as f64, 0x00 as f64, 0.0),
        Scalar::new(0xFE as f64, 0x6F as f64, 0xE5 as f64, 0.0),
        Scalar::new(0x31 as f64, 0x82 as f64, 0x78 as f64, 0.0),
        Scalar::new(0xA1 as f64, 0x4C as f64, 0x0E as f64, 0.0),
        Scalar::new(0xCB as f64, 0xD0 as f64, 0x91 as f64, 0.0),
        Scalar::new(0x70 as f64, 0x99 as f64, 0xBE as f64, 0.0),
        Scalar::new(0xE8 as f64, 0x8A as f64, 0x96 as f64, 0.0),
        Scalar::new(0x00 as f64, 0x88 as f64, 0xBB as f64, 0.0),
        Scalar::new(0x2C as f64, 0x00 as f64, 0x43 as f64, 0.0),
        Scalar::new(0x74 as f64, 0xFF as f64, 0xDE as f64, 0.0),
        Scalar::new(0xC6 as f64, 0xFF as f64, 0x00 as f64, 0.0),
        Scalar::new(0x02 as f64, 0xE5 as f64, 0xFF as f64, 0.0),
        Scalar::new(0x00 as f64, 0x0E as f64, 0x62 as f64, 0.0),
        Scalar::new(0x9C as f64, 0x8F as f64, 0x00 as f64, 0.0),
        Scalar::new(0x52 as f64, 0xFF as f64, 0x98 as f64, 0.0),
        Scalar::new(0xB1 as f64, 0x44 as f64, 0x75 as f64, 0.0),
        Scalar::new(0xFF as f64, 0x00 as f64, 0xB5 as f64, 0.0),
        Scalar::new(0x78 as f64, 0xFF as f64, 0x00 as f64, 0.0),
        Scalar::new(0x41 as f64, 0x6E as f64, 0xFF as f64, 0.0),
        Scalar::new(0x39 as f64, 0x5F as f64, 0x00 as f64, 0.0),
        Scalar::new(0x82 as f64, 0x68 as f64, 0x6B as f64, 0.0),
        Scalar::new(0x4E as f64, 0xAD as f64, 0x5F as f64, 0.0),
        Scalar::new(0x40 as f64, 0x57 as f64, 0xA7 as f64, 0.0),
        Scalar::new(0xD2 as f64, 0xFF as f64, 0xA5 as f64, 0.0),
        Scalar::new(0x67 as f64, 0xB1 as f64, 0xFF as f64, 0.0),
        Scalar::new(0xFF as f64, 0x9B as f64, 0x00 as f64, 0.0),
        Scalar::new(0xBE as f64, 0x5E as f64, 0xE8 as f64, 0.0),
    ];
    let mut result: Vec<Card> = vec![];

    // Draw rectangles around all the contours
    for c in 0..contours.len() {
        let contour: Vector<Point> = contours.get(c)?;
        let rect = min_area_rect(&contour)?;

        let mut points: [Point2f; 4] = [
            Point2f::default(),
            Point2f::default(),
            Point2f::default(),
            Point2f::default(),
        ];
        rect.points(&mut points)?;
        result.push(Card::new(
            points
                .into_iter()
                .map(|p| [p.x.round() as i32, p.y.round() as i32])
                .collect::<Vec<[i32; 2]>>(),
            contour,
        ));
    }

    // Filter rects in rects
    let mut r = 0;
    while r < result.len() {
        let c1 = &result[r];
        if result
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != r)
            .map(|(_, c2)| c2)
            .any(|c2| distance_formula(c1.x, c1.y, c2.x, c2.y) < c1.radius && c2.area > c1.area)
        {
            result.remove(r);
        } else {
            r += 1;
        }
    }

    // Draw them
    for (c, card) in result.iter().enumerate() {
        // Boxes
        for i in 0..4 {
            let a = card.rect[i];
            let b = card.rect[(i + 1) % 4];
            line(
                frame,
                Point::new(a[0], a[1]),
                Point::new(b[0], b[1]),
                colors[c % colors.len()],
                5,
                LineTypes::LINE_4 as i32,
                0,
            )?;
        }
    }

    Ok(result)
}
