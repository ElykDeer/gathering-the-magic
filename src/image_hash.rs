use anyhow::Result;
use glob::glob;

use opencv::{
    core::{
        merge, no_array, normalize as cvnormalize, split, BorderTypes, DecompTypes, Mat, NormTypes,
        Size, Vector, CV_32F,
    },
    img_hash::p_hash,
    imgcodecs::{imread, ImreadModes},
    imgproc::{
        bounding_rect, canny, contour_area, create_clahe, cvt_color, draw_contours, find_contours,
        gaussian_blur, get_perspective_transform, get_text_size, line, min_area_rect, put_text,
        resize, warp_perspective, ColorConversionCodes, ContourApproximationModes, HersheyFonts,
        InterpolationFlags, LineTypes, RetrievalModes,
    },
    prelude::{CLAHETrait, MatTraitConst, MatTraitConstManual},
};

use rayon::prelude::*;
use serde_json;
use std::{collections::HashMap, fs::File, io::BufReader};

use crate::image_card_extraction;

lazy_static::lazy_static! {
    // Example sizes I've worked with in the past. Supposedly, smaller is supposed to be better.
    // Size::new(8, 8),
    // Size::new(25, 35),
    // Size::new(50, 70),
    // Size::new(75, 105),
    // Size::new(100, 140),
    // Size::new(200, 280),
    pub(crate) static ref X: i32 = 75;
    pub(crate) static ref Y: i32 = 105;
    // pub(crate) static ref X: i32 = 250;
    // pub(crate) static ref Y: i32 = 350;
    pub(crate) static ref HASHES: HashMap<u64, String> = serde_json::from_reader(BufReader::new(File::open(database_name()).unwrap())).unwrap();
}

fn database_name() -> String {
    format!("./hashes/{}x{}.json", *X, *Y)
}

pub fn hash_all_cards() -> Result<()> {
    if std::path::Path::new(&database_name()).exists() {
        return Ok(());
    }

    println!("Calculating hases for {}x{}", *X, *Y);
    let result: HashMap<u64, String> = glob("images/*.jpg")?
        .collect::<Vec<_>>()
        .par_iter()
        .filter_map(|file| {
            let file = file.as_ref().unwrap();
            let file_name = file.file_stem().unwrap().to_str().unwrap();
            let id = &file_name[..file_name.rfind('-').unwrap()];

            // Calculate hash
            let image = imread(file.to_str().unwrap(), ImreadModes::IMREAD_COLOR as i32).unwrap();

            if let Some(hash) = calculate_hash(&image) {
                Some((hash, id.to_owned()))
            } else {
                println!("Redownload {}", file.to_str().unwrap());
                None
            }
        })
        .collect();

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(database_name())?;
    serde_json::to_writer(file, &result)?;
    println!("Done");
    Ok(())
}

pub fn normalize(frame: &Mat) -> Result<Mat> {
    // Denoise (TODO : there must be more robust ways of doing this)
    // let mut blur = Mat::default();
    // bilateral_filter(&frame, &mut blur, 9, 75.0, 75.0, BORDER_DEFAULT as i32)?;
    // let mut denoised = Mat::default();
    // fast_nl_means_denoising_colored(&blur, &mut denoised, 10.0, 10.0, 7, 21)?;

    // // CLAHE Normalization
    // let mut clahe = create_clahe(4.0, Size::new(8, 8))?;
    // let mut lab = Mat::default();
    // cvt_color(
    //     &frame,
    //     &mut lab,
    //     ColorConversionCodes::COLOR_BGR2Lab as i32,
    //     0,
    // )?;
    // let mut channels: Vector<Mat> = Vector::new();
    // split(&lab, &mut channels)?;
    // let mut dst = Mat::default();
    // clahe.apply(&channels.get(0)?, &mut dst)?;
    // channels.set(0, dst)?;
    // merge(&channels, &mut lab)?;
    // let mut normalized = Mat::default();
    // cvt_color(
    //     &lab,
    //     &mut normalized,
    //     ColorConversionCodes::COLOR_Lab2BGR as i32,
    //     0,
    // )?;

    // Builtin normalization
    // let mut normalized = Mat::default();
    // cvnormalize(
    //     &frame,
    //     &mut normalized,
    //     0.0,
    //     255.0,
    //     NormTypes::NORM_MINMAX as i32,
    //     -1,
    //     &no_array(),
    // )?;

    let mut resized_image = Mat::default();
    resize(
        &frame,
        &mut resized_image,
        Size::new(*X, *Y),
        0.0,
        0.0,
        InterpolationFlags::INTER_CUBIC as i32,
    )?;
    // Ok(resized_image)

    let mut blur = Mat::default();
    gaussian_blur(
        &resized_image,
        &mut blur,
        Size::new(5, 5),
        0.0,
        0.0,
        opencv::core::BORDER_DEFAULT,
    )?;
    Ok(blur)
}

pub fn calculate_hash(image: &Mat) -> Option<u64> {
    if let Ok(normalized) = normalize(image) {
        let mut hash = Mat::default();
        p_hash(&normalized, &mut hash).unwrap();
        Some(u64::from_be_bytes(
            hash.data_bytes().unwrap().try_into().unwrap(),
        ))
    } else {
        None
    }
}

pub fn hamming_distance(hash: u64, hash1: u64) -> u64 {
    let mut x = hash ^ hash1;
    let mut distance = 0;

    while x > 0 {
        distance += x & 1;
        x >>= 1;
    }

    distance
}

pub(crate) fn get_card_id(frame: &Mat) -> Option<String> {
    let card_image = {
        let card = image_card_extraction::CARD.lock().unwrap();
        if let Ok(card_image) = (*card).get_unwarped(frame) {
            Some(card_image.clone())
        } else {
            None
        }
    };

    if let Some(card_image) = card_image {
        let src_hash = calculate_hash(&card_image).unwrap();

        // Calculate distance to card in db
        let distances: HashMap<u64, u64> = HASHES
            .par_iter()
            .map(|(&oracle_hash, _)| (oracle_hash, hamming_distance(src_hash, oracle_hash)))
            .collect();

        let (dest_hash, _) = distances
            .par_iter()
            .min_by(|&(_, distance1), &(_, distance2)| distance1.cmp(distance2))
            .unwrap();

        Some(HASHES[dest_hash].clone())
    } else {
        None
    }
}
