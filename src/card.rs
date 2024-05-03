use anyhow::Result;
use opencv::{
    core::{BorderTypes, DecompTypes, Mat, Point, Point2f, Scalar, Size, Vector},
    imgproc::{
        contour_area, get_perspective_transform, line, warp_perspective, InterpolationFlags,
        LineTypes,
    },
};
use std::time::SystemTime;

pub(crate) fn distance_formula(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    (((x2 - x1) as f64).powi(2) + ((y2 - y1) as f64).powi(2)).sqrt()
}

pub struct Card {
    pub rect: [[i32; 2]; 4],
    pub last_seen: SystemTime,
    pub alive: bool,
    pub processed: bool,
    pub x: i32,
    pub y: i32,
    pub radius: f64,
    pub area: f64,
    pub contour: Vector<Point>,
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
            processed: false,
            x,
            y,
            radius,
            area,
            contour,
        }
    }

    pub fn update(&mut self, new_card: Card) {
        if self.alive && distance_formula(new_card.x, new_card.y, self.x, self.y) < self.radius {
            self.rect = new_card.rect;
            self.last_seen = new_card.last_seen;
            self.x = new_card.x;
            self.y = new_card.y;
            self.radius = new_card.radius;
            self.area = new_card.area;
            self.contour = new_card.contour;
        } else {
            *self = new_card;
        }
    }

    // Set myself as stale if we haven't seen anything a while
    pub fn prune(&mut self) {
        if self.alive && self.last_seen.elapsed().unwrap().as_secs_f64() > 1.0 {
            println!("Death");
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

    fn get_closest_contour_point(&self, x: i32, y: i32) -> [i32; 2] {
        let point = self
            .contour
            .iter()
            .min_by(|point_1, point_2| {
                distance_formula(point_1.x, point_1.y, x, y)
                    .total_cmp(&distance_formula(point_2.x, point_2.y, x, y))
            })
            .unwrap();
        [point.x, point.y]
    }

    pub fn get_unwarped(&self, frame: &Mat) -> Result<Mat> {
        // std::vector<cv::Point2f> src(4);
        let corner_1 = self.get_closest_contour_point(self.rect[0][0], self.rect[0][1]);
        let corner_2 = self.get_closest_contour_point(self.rect[1][0], self.rect[1][1]);
        let corner_3 = self.get_closest_contour_point(self.rect[2][0], self.rect[2][1]);
        let corner_4 = self.get_closest_contour_point(self.rect[3][0], self.rect[3][1]);
        let old_corners: Vector<Point2f> = vec![
            Point2f::new(corner_1[0] as f32, corner_1[1] as f32),
            Point2f::new(corner_2[0] as f32, corner_2[1] as f32),
            Point2f::new(corner_3[0] as f32, corner_3[1] as f32),
            Point2f::new(corner_4[0] as f32, corner_4[1] as f32),
        ]
        .into();

        let side_1_length = distance_formula(
            self.rect[0][0],
            self.rect[0][1],
            self.rect[1][0],
            self.rect[1][1],
        )
        .round() as i32;
        let side_2_length = distance_formula(
            self.rect[1][0],
            self.rect[1][1],
            self.rect[2][0],
            self.rect[2][1],
        )
        .round() as i32;

        let width = std::cmp::min(side_1_length, side_2_length);
        let height = std::cmp::max(side_1_length, side_2_length);

        // std::vector<cv::Point2f> dst(4);
        let new_corners: Vector<Point2f> = if width == side_1_length {
            vec![
                Point2f::new(0.0, 0.0),
                Point2f::new(width as f32 - 1.0, 0.0),
                Point2f::new(width as f32 - 1.0, height as f32 - 1.0),
                Point2f::new(0.0, height as f32 - 1.0),
            ]
            .into()
        } else {
            vec![
                Point2f::new(0.0, height as f32 - 1.0),
                Point2f::new(0.0, 0.0),
                Point2f::new(width as f32 - 1.0, 0.0),
                Point2f::new(width as f32 - 1.0, height as f32 - 1.0),
            ]
            .into()
        };

        let transform =
            get_perspective_transform(&old_corners, &new_corners, DecompTypes::DECOMP_LU as i32)?;

        let mut result = Mat::default();
        warp_perspective(
            frame,
            &mut result,
            &transform,
            Size::new(width, height),
            InterpolationFlags::INTER_LINEAR as i32,
            BorderTypes::BORDER_CONSTANT as i32,
            Scalar::default(),
        )?;
        Ok(result)
    }
}

impl Default for Card {
    fn default() -> Self {
        Self {
            rect: <[_; 4]>::default(),
            last_seen: SystemTime::now(),
            alive: false,
            processed: false,
            x: 0,
            y: 0,
            radius: 0.0,
            area: 0.0,
            contour: Vector::default(),
        }
    }
}
