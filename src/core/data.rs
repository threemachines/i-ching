use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigram {
    pub name: String,
    pub chinese: String,
    pub unicode: String,
    pub symbolic: String,
    pub element: String,
    pub attribute: String,
    pub lines: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexagramJudgment {
    pub text: String,
    pub commentary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexagramImage {
    pub text: String,
    pub commentary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineInterpretation {
    pub text: String,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hexagram {
    pub number: u8,
    pub name: String,
    pub chinese: String,
    pub pinyin: String,
    pub unicode: String,
    pub binary: String,
    pub opposite: String,
    pub upper_trigram: String,
    pub lower_trigram: String,
    pub description: String,
    pub judgment: HexagramJudgment,
    pub image: HexagramImage,
    pub lines: HashMap<String, LineInterpretation>,
}

pub struct IChingData {
    pub trigrams: HashMap<String, Trigram>,
    pub hexagrams: HashMap<String, Hexagram>,
}

impl IChingData {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load from embedded data first, then fall back to files
        Self::load_embedded().or_else(|_| Self::load_from_files())
    }

    /// Load data embedded in the binary at compile time
    fn load_embedded() -> Result<Self, Box<dyn std::error::Error>> {
        // Embed the JSON files at compile time
        let trigrams_content = include_str!("../../data/trigrams.json");
        let hexagrams_content = include_str!("../../data/hexagrams.json");

        let trigrams: HashMap<String, Trigram> = serde_json::from_str(trigrams_content)?;
        let hexagrams: HashMap<String, Hexagram> = serde_json::from_str(hexagrams_content)?;

        Ok(IChingData {
            trigrams,
            hexagrams,
        })
    }

    /// Load data from external files (fallback for development)
    fn load_from_files() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = Self::find_data_directory()?;

        // Load trigrams
        let trigrams_path = data_dir.join("trigrams.json");
        let trigrams_content = fs::read_to_string(&trigrams_path).map_err(|e| {
            format!(
                "Failed to read trigrams.json from {}: {}",
                trigrams_path.display(),
                e
            )
        })?;
        let trigrams: HashMap<String, Trigram> = serde_json::from_str(&trigrams_content)?;

        // Load hexagrams
        let hexagrams_path = data_dir.join("hexagrams.json");
        let hexagrams_content = fs::read_to_string(&hexagrams_path).map_err(|e| {
            format!(
                "Failed to read hexagrams.json from {}: {}",
                hexagrams_path.display(),
                e
            )
        })?;
        let hexagrams: HashMap<String, Hexagram> = serde_json::from_str(&hexagrams_content)?;

        Ok(IChingData {
            trigrams,
            hexagrams,
        })
    }

    fn find_data_directory() -> Result<PathBuf, Box<dyn std::error::Error>> {
        // Try multiple locations in order of preference
        let candidates = vec![
            // 1. Current working directory
            PathBuf::from("data"),
            // 2. Relative to the executable
            env::current_exe()?.parent().unwrap().join("data"),
            // 3. Relative to the executable's parent (for development)
            env::current_exe()?
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .join("data"),
            // 4. In the same directory as the executable
            env::current_exe()?.parent().unwrap().to_path_buf(),
        ];

        for candidate in candidates {
            if candidate.join("trigrams.json").exists() && candidate.join("hexagrams.json").exists()
            {
                return Ok(candidate);
            }
        }

        Err("Could not find data directory with trigrams.json and hexagrams.json. Please ensure the data files are in one of these locations: ./data/, next to the executable, or in the parent directory.".into())
    }

    pub fn get_hexagram(&self, number: u8) -> Option<&Hexagram> {
        self.hexagrams.get(&number.to_string())
    }

    pub fn get_trigram(&self, name: &str) -> Option<&Trigram> {
        self.trigrams.get(name)
    }

    pub fn get_line_interpretation(
        &self,
        hexagram_number: u8,
        line_position: u8,
    ) -> Option<&LineInterpretation> {
        self.get_hexagram(hexagram_number)?
            .lines
            .get(&line_position.to_string())
    }
}
