use crate::card;
use crate::search;
use crate::text_extraction::extract_text_from_mat;

use anyhow::Result;
use opencv::{
    core::{Mat, MatTraitConst, Point, Point2f, Size, Vector},
    imgcodecs::{imdecode, IMREAD_COLOR},
    imgproc::{
        bounding_rect, cvt_color, find_contours, gaussian_blur, min_area_rect, threshold,
        ContourApproximationModes, RetrievalModes, COLOR_BGR2GRAY, THRESH_BINARY,
    },
};

lazy_static::lazy_static! {
    pub(crate) static ref CARD: std::sync::Mutex<card::Card> = std::sync::Mutex::new(card::Card::default());
}

pub(crate) fn process_frame(frame_data: &[u8]) -> Result<Option<String>> {
    let mut frame = imdecode(&Vector::from_slice(frame_data), IMREAD_COLOR)?;

    let (alive, processed) = {
        let mut card = CARD.lock().unwrap();
        if let Some(new_card) = get_card(&mut frame)? {
            card.update(new_card);
        }
        card.prune();
        card.draw(&mut frame);
        (card.alive, card.processed)
    };

    let mut results = None;
    if alive && !processed {
        // Extract tokens
        if let Ok(text) = extract_text_from_mat(&frame) {
            // Filter to tokens in our dataset
            let text = search::filter_string(text);
            if !text.is_empty() {
                // Get top 30 card matches
                results = Some(search(&text));
                println!("Got search results for `{}`.", &text);
                // TODO : Change search function to return IDs?
                // TODO : Add another function which converts IDs to the final format

                let mut card = CARD.lock().unwrap();
                card.processed = true;
            }
        }

        // TODO : Auto accept first result
        // CONT :   I was hoping to just swipe it off frame
        // CONT :     but that would interact poorly with the current rejection code / results speed
        // CONT :   I was hoping to use the death signal as the accept signal
        // CONT :     and to do that I'd need to add a different rejection signal/timer/handler
        // CONT :   One idea I've had is to explicitly reject the IDs of the results
        // CONT :     and filter future results instead of just making a new query
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
    Ok(results)
}

/// This function should take the raw camera image and normalize it for contour extraction
fn camera_normalization(frame: &Mat) -> Result<Mat> {
    // Convert image to grayscale
    let mut gray = Mat::default();
    cvt_color(frame, &mut gray, COLOR_BGR2GRAY, 0)?;

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

    // Apply binary threshold
    let mut thresh = Mat::default();
    threshold(&gray, &mut thresh, 80.0, 255.0, THRESH_BINARY)?;

    Ok(thresh)
}

/// Given a frame of video, this'll try to identify a contrasting rectangular object in the screen, and initialize a Card object for it
fn get_card(frame: &mut Mat) -> Result<Option<card::Card>> {
    let normalized_camera_input = camera_normalization(frame)?;

    // let mut can = Mat::default();
    // canny(frame, &mut can, 100.0, 200.0, 3, false)?;

    let mut contours: Vector<Vector<Point>> = Vector::new();
    find_contours(
        &normalized_camera_input,
        &mut contours,
        RetrievalModes::RETR_LIST as i32,
        ContourApproximationModes::CHAIN_APPROX_TC89_KCOS as i32,
        Point::default(),
    )?;

    // Prune Contours
    let frame_area = frame.size()?.width * frame.size()?.height;
    let min_area = frame_area as f64 * 0.2;
    let max_area = frame_area as f64 * 0.5;
    let contours: Vector<Vector<Point>> = contours
        .into_iter()
        .filter(|c| {
            let area = opencv::imgproc::contour_area(&c, false).unwrap();

            // Filter contours based on the area size
            if min_area < area && area < max_area {
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
