use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};

lazy_static::lazy_static! {
    pub(crate) static ref CARD_DATABASE: std::sync::Mutex<CardDatabase> = std::sync::Mutex::new(CardDatabase::load().unwrap_or_default());
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct CardDatabase {
    database: HashMap<String, usize>,
    history: Vec<(String, String, usize)>,
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

    fn record_change(&mut self, id: String, change_type: &str) {
        let new_value = self.database[&id];
        self.history.push((id, change_type.to_string(), new_value));
    }

    pub(crate) fn inc(&mut self, id: &str) {
        let count = self.database.entry(id.to_string()).or_insert(0);
        *count += 1;
        self.record_change(id.to_string(), "inc");
        if let Err(e) = self.save() {
            eprintln!("Failed to save data: {}", e);
        }
    }

    pub(crate) fn dec(&mut self, id: &str) {
        if let Some(count) = self.database.get_mut(id) {
            if *count > 0 {
                *count -= 1;
                self.record_change(id.to_string(), "dec");
                if let Err(e) = self.save() {
                    eprintln!("Failed to save data: {}", e);
                }
            }
        }
    }

    pub(crate) fn set(&mut self, id: &str, value: usize) {
        self.database.insert(id.to_string(), value);
        self.record_change(id.to_string(), "set");
        if let Err(e) = self.save() {
            eprintln!("Failed to save data: {}", e);
        }
    }

    pub(crate) fn get_recent_changes(&self) -> Vec<String> {
        self.history
            .iter()
            .rev()
            .take(30)
            .map(|(id, _, _)| id.clone())
            .collect()
    }

    pub(crate) fn get(&self, id: &str) -> usize {
        self.database.get(id).cloned().unwrap_or(0)
    }
}
