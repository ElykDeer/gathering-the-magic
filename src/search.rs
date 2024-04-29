use lazy_static::lazy_static;
use scryers::{
    bulk::{BulkDownload, BulkDownloadType},
    card::Card,
};
use std::{cmp::Ordering, collections::BinaryHeap};
use strsim::jaro_winkler;

lazy_static! {
    pub(crate) static ref CARDS: std::sync::Mutex<BulkDownload> = {
        std::sync::Mutex::new(
            BulkDownload::new("./scryfall.db", BulkDownloadType::UniqueArtwork).unwrap(),
        )
    };
    pub(crate) static ref ALL_FILES: Vec<String> =
        std::fs::read_dir(std::path::Path::new("../gathering_the_magic/images/"))
            .unwrap()
            .map(|entry| {
                entry
                    .unwrap()
                    .path()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
}

struct ScoredCard<'a> {
    score: f64,
    card: &'a Card,
}

impl<'a> PartialEq for ScoredCard<'a> {
    fn eq(&self, other: &Self) -> bool {
        (self.score - other.score).abs() < f64::EPSILON
    }
}

impl<'a> Eq for ScoredCard<'a> {}

impl<'a> Ord for ScoredCard<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

impl<'a> PartialOrd for ScoredCard<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.score.partial_cmp(&self.score)
    }
}

fn rank(query: &str) -> Vec<String> {
    let mut scryer = CARDS.lock().unwrap();
    let mut heap = BinaryHeap::new();
    for card in scryer.cards() {
        let scores = [
            jaro_winkler(&card.name().to_lowercase(), &query.to_lowercase()),
            card.oracle_text()
                .as_ref()
                .map(|text| jaro_winkler(&text.to_lowercase(), &query.to_lowercase()))
                .unwrap_or(0.0),
            card.type_line()
                .as_ref()
                .map(|type_line| jaro_winkler(&type_line.to_lowercase(), &query.to_lowercase()))
                .unwrap_or(0.0),
            card.keywords().iter().fold(0.0, |max, keyword| {
                let score = jaro_winkler(&keyword.to_lowercase(), &query.to_lowercase());
                if score > max {
                    score
                } else {
                    max
                }
            }),
            card.artist()
                .as_ref()
                .map(|artist| jaro_winkler(&artist.to_lowercase(), &query.to_lowercase()))
                .unwrap_or(0.0),
            card.flavor_name()
                .as_ref()
                .map(|flavor_name| jaro_winkler(&flavor_name.to_lowercase(), &query.to_lowercase()))
                .unwrap_or(0.0),
            card.flavor_text()
                .as_ref()
                .map(|flavor_text| jaro_winkler(&flavor_text.to_lowercase(), &query.to_lowercase()))
                .unwrap_or(0.0),
            card.set_name()
                .as_ref()
                .map(|set_name| jaro_winkler(&set_name.to_lowercase(), &query.to_lowercase()))
                .unwrap_or(0.0),
        ];
        let max_score = scores.iter().cloned().fold(0.0, f64::max);

        heap.push(ScoredCard {
            score: max_score,
            card: &card,
        });

        if heap.len() > 30 {
            heap.pop();
        }
    }
    heap.into_sorted_vec()
        .into_iter()
        .map(|scored| {
            // println!("{} : {}", scored.card.name(), scored.score);
            scored.card.id().to_owned()
        })
        .collect()
}

pub(crate) fn search(query: &str) -> String {
    let ids = rank(query);

    let cards: Vec<(String, String, String)> = {
        let mut scryer = CARDS.lock().unwrap();
        ids.into_iter()
            .map(|id| {
                ALL_FILES
                    .iter()
                    .filter(|filename| filename.starts_with(&id))
                    .map(|filename| {
                        let card = scryer.get_card_by_id(&id).unwrap();
                        (id.clone(), filename.clone(), card.name().to_owned())
                    })
                    .collect::<Vec<(String, String, String)>>()
            })
            .flatten()
            .collect()
    };

    cards
        .into_iter()
        .map(|(id, img, name)| {
            format!(
                r#"{{"imageUrl": "/images/{}", "uuid": "{}", "name": "{}"}}"#,
                img, id, name,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}
