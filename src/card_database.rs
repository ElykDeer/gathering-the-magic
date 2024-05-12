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
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct CardDatabase {
    database: HashMap<String, usize>,
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
            // file_to_id,
        }))
    }

    fn save(&self) -> io::Result<()> {
        // Rename the old file as a backup
        if fs::metadata("./database.json").is_ok() {
            fs::rename("./database.json", "./database.json.bak")?;
        }

        // Serialize the database to a temporary file
        let mut file = File::create("./database.json")?;
        file.write_all(serde_json::to_string_pretty(&self)?.as_bytes())?;
        file.sync_all()?;
        fs::remove_file("./database.json.bak")
    }

    fn record_change(&mut self, file_name: String, change_type: ChangeType) {
        let new_value = self.database[&file_name];
        let entry = HistoryEntry {
            file_name,
            change_type,
            updated_value: new_value,
        };
        self.history.push(entry);
    }

    pub(crate) fn inc(&mut self, id: &str) {
        let count = self.database.entry(id.to_string()).or_insert(0);
        *count += 1;
        self.record_change(id.to_string(), ChangeType::Inc);
        if let Err(e) = self.save() {
            eprintln!("Failed to save data: {}", e);
        }
    }

    pub(crate) fn dec(&mut self, id: &str) {
        if let Some(count) = self.database.get_mut(id) {
            if *count > 0 {
                *count -= 1;
                self.record_change(id.to_string(), ChangeType::Dec);
                if let Err(e) = self.save() {
                    eprintln!("Failed to save data: {}", e);
                }
            }
        }
    }

    pub(crate) fn set(&mut self, id: &str, value: usize) {
        self.database.insert(id.to_string(), value);
        self.record_change(id.to_string(), ChangeType::Set);
        if let Err(e) = self.save() {
            eprintln!("Failed to save data: {}", e);
        }
    }

    pub(crate) fn history(&self) -> (usize, f64, String) {
        let total_cards = self.database.values().sum();
        let total_value = {
            let mut scryrs = crate::search::CARDS.lock().unwrap();

            self.database.iter().fold(0.0, |prev, (uuid, count)| {
                prev + scryrs
                    .get_card_by_id(&uuid[..uuid.rfind('-').unwrap()])
                    .unwrap()
                    .usd()
                    * *count as f64
            })
        };

        let mut seen_files = HashSet::new();
        let cards = {
            self.history
                .iter()
                .rev()
                .take(120)
                .filter(|history_entry| seen_files.insert(history_entry.file_name.clone()))
                .take(60)
                .map(|history_entry| {
                    format!(
                        r#"{{"uuid": "{}", "count": "{}"}}"#,
                        &history_entry.file_name,
                        self.get(&history_entry.file_name)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };

        (total_cards, total_value, cards)
    }

    pub(crate) fn get(&self, id: &str) -> usize {
        self.database.get(id).cloned().unwrap_or(0)
    }
}
