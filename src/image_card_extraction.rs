use crate::card;

use anyhow::Result;
use opencv::{
    core::Vector,
    imgcodecs::{imdecode, IMREAD_COLOR},
};
use opencv::{
    core::{Mat, Point, Size},
    imgproc::{
        bounding_rect, cvt_color, find_contours, gaussian_blur, threshold,
        ContourApproximationModes, RetrievalModes, COLOR_BGR2GRAY, THRESH_BINARY,
    },
};

pub(crate) fn process_frame(frame_data: &[u8]) -> Result<()> {
    let mut frame = imdecode(&Vector::from_slice(frame_data), IMREAD_COLOR)?;

    get_cards(&mut frame)?;

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

pub fn get_cards(frame: &mut Mat) -> Result<Vec<card::Card>> {
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

    card::find_cards(&contours, frame)
}
