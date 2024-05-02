use crate::card;
use crate::image_hash;
use crate::search;
use crate::text_extraction::extract_text_from_mat;

use anyhow::Result;
use opencv::prelude::MatConstIteratorTraitManual;
use opencv::{
    core::{Mat, Point, Point2f, Size, Vector},
    imgcodecs::{imdecode, IMREAD_COLOR},
    imgproc::{
        bounding_rect, cvt_color, find_contours, gaussian_blur, min_area_rect, threshold,
        ContourApproximationModes, RetrievalModes, COLOR_BGR2GRAY, THRESH_BINARY,
    },
};

lazy_static::lazy_static! {
    pub(crate) static ref CARD: std::sync::Mutex<card::Card> = std::sync::Mutex::new(card::Card::default());
}

pub(crate) fn process_frame(frame_data: &[u8]) -> Result<()> {
    let mut frame = imdecode(&Vector::from_slice(frame_data), IMREAD_COLOR)?;

    let alive = {
        let mut card = CARD.lock().unwrap();
        if let Some(new_card) = get_card(&mut frame)? {
            card.update(new_card);
        }
        card.prune();
        card.draw(&mut frame);
        card.alive
    };

    if alive {
        // Extract tokens
        if let Ok(text) = extract_text_from_mat(&frame) {
            // Filter to tokens in our dataset
            let text = search::filter_string(text);
            if !text.is_empty() {
                // Get top 30 card matches
                let results = search(&text);
                // TODO : Change search function to return IDs?
                // TODO : Add another function which converts IDs to the final format

                // TODO : Rank top 30 search results by image hash distance
                // if let Some(card_id) = image_hash::get_card_id(&frame) {
                //     let mut cards = crate::search::CARDS.lock().unwrap();
                //     let card = cards.get_card_by_id(&card_id).unwrap();
                //     println!("Card: {}", card.name());
                // }

                // TODO : Send the guessed ID/variation to the client to accept or not (finish UI)
                // let reply = Message::text(format!(
                //     r#"{{"action": "searchResults", "results": [{}]}}"#,
                //     results
                // ));
                // assert!(tx.send(reply).await.is_ok());
            }
        }

        // TODO : Database stuff
        // TODO : If DEATH comes before REJECTION, "save to database" (print for now)
        // TODO : Start testing
    }

    // Send the frame to the visualizer, if the visualizer is enabled
    if let (Ok(visualizer), Ok(mut global_frame)) = (
        crate::image::VISUALIZER_ENABLED.try_lock(),
        crate::image::CURRENT_FRAME.try_lock(),
    ) {
        if *visualizer {
            *global_frame = frame;
        }
    }
    Ok(())
}

pub fn get_card(frame: &mut Mat) -> Result<Option<card::Card>> {
    // Convert image to grayscale
    let mut gray = Mat::default();
    cvt_color(frame, &mut gray, COLOR_BGR2GRAY, 0)?;
    // *frame = gray.clone();

    // Apply Gaussian blur
    let mut blur = Mat::default();
    gaussian_blur(
        &gray,
        &mut blur,
        Size::new(5, 5),
        0.0,
        0.0,
        opencv::core::BORDER_DEFAULT,
    )?;
    // *frame = blur.clone();

    // Apply binary threshold
    let mut thresh = Mat::default();
    threshold(&gray, &mut thresh, 80.0, 255.0, THRESH_BINARY)?;
    // *frame = thresh.clone();

    // let mut can = Mat::default();
    // canny(frame, &mut can, 100.0, 200.0, 3, false)?;
    // *frame = can.clone();

    let mut contours: Vector<Vector<Point>> = Vector::new();
    find_contours(
        &thresh,
        &mut contours,
        RetrievalModes::RETR_LIST as i32,
        ContourApproximationModes::CHAIN_APPROX_TC89_KCOS as i32,
        Point::default(),
    )?;

    // Prune Contours
    let contours: Vector<Vector<Point>> = contours
        .into_iter()
        .filter(|c| {
            let area = opencv::imgproc::contour_area(&c, false).unwrap();

            // Filter contours based on the area size
            // average = 1_745_674
            // min = 900_577
            // max = 2_381_695
            if area > 1_800_000.0 && area < 5_000_000.0 {
                let peri = opencv::imgproc::arc_length(&c, true).unwrap();
                let mut approx: Vector<Point> = Vector::new();
                opencv::imgproc::approx_poly_dp(&c, &mut approx, 0.02 * peri, true).unwrap();

                if approx.len() == 4 {
                    let bounding_rect = bounding_rect(&c).unwrap();
                    let aspect_ratio = std::cmp::max(bounding_rect.width, bounding_rect.height)
                        as f64
                        / std::cmp::min(bounding_rect.width, bounding_rect.height) as f64;

                    // Check if the aspect ratio is close to 1.4
                    if (1.1..1.7).contains(&aspect_ratio) {
                        return true;
                    }
                }
            }
            false
        })
        .collect();

    find_card(&contours)
}

fn find_card(contours: &Vector<Vector<Point>>) -> Result<Option<card::Card>> {
    let mut result: Vec<card::Card> = vec![];

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
        result.push(card::Card::new(
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
            .any(|c2| {
                card::distance_formula(c1.x, c1.y, c2.x, c2.y) < c1.radius && c2.area > c1.area
            })
        {
            result.remove(r);
        } else {
            r += 1;
        }
    }

    if result.is_empty() {
        Ok(None)
    } else {
        Ok(Some(result.into_iter().next().unwrap()))
    }
}
