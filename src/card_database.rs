use serde::{Deserialize, Serialize};
use serde_json;
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{self, Write},
};

lazy_static::lazy_static! {
    pub(crate) static ref CARD_DATABASE: std::sync::Mutex<CardDatabase> = std::sync::Mutex::new(CardDatabase::load().unwrap_or_default());
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ChangeType {
    Inc,
    Dec,
    Set,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct HistoryEntry {
    pub(crate) file_name: String,
    change_type: ChangeType,
    updated_value: usize,
    foil: bool,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct CardCounts {
    non_foil: usize,
    foil: usize,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct CardDatabase {
    database: HashMap<String, CardCounts>,
    pub(crate) history: Vec<HistoryEntry>,
}

impl CardDatabase {
    fn load() -> io::Result<Self> {
        let file = fs::File::open("./database.json").map_err(|err| {
            if err.kind() == io::ErrorKind::NotFound {
                File::create("./database.json").unwrap();
                io::Error::new(io::ErrorKind::NotFound, "File created")
            } else {
                err
            }
        })?;

        serde_json::from_reader(file).or(Ok(Self {
            database: HashMap::new(),
            history: Vec::new(),
        }))
    }

    fn save(&self) -> io::Result<()> {
        if fs::metadata("./database.json").is_ok() {
            fs::rename("./database.json", "./database.json.bak")?;
        }

        let mut file = File::create("./database.json")?;
        file.write_all(serde_json::to_string_pretty(&self)?.as_bytes())?;
        file.sync_all()?;
        fs::remove_file("./database.json.bak")
    }

    fn record_change(&mut self, file_name: String, change_type: ChangeType, foil: bool) {
        let card_counts = &self.database[&file_name];
        let updated_value = if foil {
            card_counts.foil
        } else {
            card_counts.non_foil
        };
        let entry = HistoryEntry {
            file_name,
            change_type,
            updated_value,
            foil,
        };
        self.history.push(entry);
    }

    pub(crate) fn inc(&mut self, id: &str, foil: bool) -> bool {
        let counts = self
            .database
            .entry(id.to_string())
            .or_insert(CardCounts::default());
        let result = if foil {
            counts.foil += 1;
            counts.foil == 1
        } else {
            counts.non_foil += 1;
            counts.non_foil == 1
        };

        self.record_change(id.to_string(), ChangeType::Inc, foil);
        if let Err(e) = self.save() {
            eprintln!("Failed to save data: {}", e);
        }

        result
    }

    pub(crate) fn dec(&mut self, id: &str, foil: bool) {
        if let Some(counts) = self.database.get_mut(id) {
            if foil && counts.foil > 0 {
                counts.foil -= 1;
            } else if !foil && counts.non_foil > 0 {
                counts.non_foil -= 1;
            }
            self.record_change(id.to_string(), ChangeType::Dec, foil);
            if let Err(e) = self.save() {
                eprintln!("Failed to save data: {}", e);
            }
        }
    }

    pub(crate) fn set(&mut self, id: &str, value: usize, foil: bool) {
        if foil {
            self.database
                .entry(id.to_string())
                .or_insert(CardCounts::default())
                .foil = value;
        } else {
            self.database
                .entry(id.to_string())
                .or_insert(CardCounts::default())
                .non_foil = value;
        }
        self.record_change(id.to_string(), ChangeType::Set, foil);
        if let Err(e) = self.save() {
            eprintln!("Failed to save data: {}", e);
        }
    }

    pub(crate) fn history(&self) -> (usize, f64, String) {
        let total_cards = self.database.values().map(|c| c.non_foil + c.foil).sum();
        let total_value = {
            let scryrs = crate::search::CARDS.lock().unwrap();

            self.database.iter().fold(0.0, |prev, (uuid, counts)| {
                let card = scryrs
                    .get_card_by_id(&uuid[..uuid.rfind('-').unwrap()])
                    .unwrap();
                prev + card.usd() * (counts.non_foil as f64)
                    + card.usd_foil() * (counts.foil as f64)
            })
        };

        let mut seen_files = HashSet::new();
        let cards = {
            let scryrs = crate::search::CARDS.lock().unwrap();
            self.history
                .iter()
                .rev()
                .take(120)
                .filter(|history_entry| seen_files.insert(history_entry.file_name.clone()))
                .take(60)
                .map(|history_entry| {
                    format!(
                        r#"{{"uuid": "{}", "non_foil_count": "{}", "foil_count": "{}", "value": "{:.2}"}}"#,
                        &history_entry.file_name,
                        self.get(&history_entry.file_name),
                        self.get_foil(&history_entry.file_name),
                        scryrs.get_card_by_id(&history_entry.file_name[..history_entry.file_name.rfind('-').unwrap()]).unwrap().usd()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        (total_cards, total_value, cards)
    }

    pub(crate) fn get(&self, id: &str) -> usize {
        self.database.get(id).map_or(0, |c| c.non_foil)
    }

    pub(crate) fn get_foil(&self, id: &str) -> usize {
        self.database.get(id).map_or(0, |c| c.foil)
    }
}
