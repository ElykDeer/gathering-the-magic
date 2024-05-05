use lazy_static::lazy_static;
use scryers::{
    bulk::{BulkDownload, BulkDownloadType},
    card::Card,
};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
    sync::Mutex,
};
use strsim::jaro_winkler;

lazy_static! {
    pub(crate) static ref CARDS: Mutex<BulkDownload> =
        Mutex::new(BulkDownload::new("./scryfall.db", BulkDownloadType::DefaultCards).unwrap());
    pub(crate) static ref ID_TO_FILES: Mutex<HashMap<String, Vec<String>>> = {
        let all_files: HashSet<String> = std::fs::read_dir(std::path::Path::new("./images/"))
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

        let mut cards = BulkDownload::new("./scryfall.db", BulkDownloadType::DefaultCards).unwrap();
        Mutex::new(
            cards
                .cards()
                .iter()
                .map(|card| {
                    (
                        card.id().to_owned(),
                        all_files
                            .iter()
                            .filter_map(|filename| {
                                if filename.starts_with(card.id()) {
                                    Some(filename.to_owned())
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    )
                })
                .collect(),
        )
    };
    pub(crate) static ref TOKENS: Mutex<HashSet<String>> = {
        let mut cards = CARDS.lock().unwrap();
        let mut tokens = HashSet::new();
        for card in cards.cards().iter() {
            if card.lang() != "en" {
                continue;
            }

            tokens.extend(
                card.name()
                    .to_lowercase()
                    .split_whitespace()
                    .map(String::from),
            );
            if let Some(text) = card.oracle_text() {
                tokens.extend(text.to_lowercase().split_whitespace().map(String::from));
            }
            if let Some(type_line) = card.type_line() {
                tokens.extend(
                    type_line
                        .to_lowercase()
                        .split_whitespace()
                        .map(String::from),
                );
            }
            tokens.extend(card.keywords().iter().map(String::from));
            if let Some(flavor_name) = card.flavor_name() {
                tokens.extend(
                    flavor_name
                        .to_lowercase()
                        .split_whitespace()
                        .map(String::from),
                );
            }
            if let Some(flavor_text) = card.flavor_text() {
                tokens.extend(
                    flavor_text
                        .to_lowercase()
                        .split_whitespace()
                        .map(String::from),
                );
            }

            if let Some(set_name) = card.set_name() {
                tokens.extend(set_name.to_lowercase().split_whitespace().map(String::from));
            }
        }

        Mutex::new(tokens)
    };
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
    let mut cards = CARDS.lock().unwrap();
    let mut heap = BinaryHeap::new();
    for card in cards.cards() {
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
            card.flavor_name()
                .as_ref()
                .map(|flavor_name| jaro_winkler(&flavor_name.to_lowercase(), &query.to_lowercase()))
                .unwrap_or(0.0),
            card.flavor_text()
                .as_ref()
                .map(|flavor_text| jaro_winkler(&flavor_text.to_lowercase(), &query.to_lowercase()))
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
        .map(|scored| scored.card.id().to_owned())
        .collect()
}

pub(crate) fn search(query: &str) -> String {
    let ids = rank(query);
    // let mut cards = CARDS.lock().unwrap();
    ids.into_iter()
        .map(|id| {
            ID_TO_FILES
                .lock()
                .unwrap()
                .get(&id)
                .unwrap()
                .iter()
                // .map(|filename| {
                //     let card = cards.get_card_by_id(&id).unwrap();
                //     (filename.clone(), card.name().to_owned())
                // })
                // .collect::<Vec<(String, String)>>()
                .cloned()
                .collect::<Vec<String>>()
        })
        .flatten()
        // .map(|(img, name)| format!(r#"{{"uuid": "{}", "count": "{}"}}"#, img, name, count))
        .map(|uuid| format!(r#"{{"uuid": "{}"}}"#, uuid))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn filter_string(input: String) -> String {
    input
        .split_whitespace()
        .filter(|&token| TOKENS.lock().unwrap().contains(&token.to_lowercase()))
        .take(4) // TODO : I'm going to forget about this and it's going to be a problem
        .collect::<Vec<&str>>()
        .join(" ")
}
