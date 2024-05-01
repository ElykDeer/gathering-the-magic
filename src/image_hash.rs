use anyhow::Result;
use glob::glob;
use opencv::{
    core::{Mat, Size},
    img_hash::p_hash,
    imgcodecs::{imread, ImreadModes},
    imgproc::{gaussian_blur, resize, InterpolationFlags},
    prelude::MatTraitConstManual,
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

pub fn calculate_hash(image: &Mat) -> Option<u64> {
    let mut blur = Mat::default();
    if gaussian_blur(
        &image,
        &mut blur,
        Size::new(5, 5),
        0.0,
        0.0,
        opencv::core::BORDER_DEFAULT,
    )
    .is_err()
    {
        return None;
    }

    let mut resized_image = Mat::default();
    if resize(
        &blur,
        &mut resized_image,
        Size::new(*X, *Y),
        0.0,
        0.0,
        InterpolationFlags::INTER_LINEAR as i32,
    )
    .is_err()
    {
        return None;
    }

    let mut hash = Mat::default();
    p_hash(&resized_image, &mut hash).unwrap();
    Some(u64::from_be_bytes(
        hash.data_bytes().unwrap().try_into().unwrap(),
    ))
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
