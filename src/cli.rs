use crate::core::data::IChingData;
use crate::core::{Diviner, Reading};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonHexagram {
    pub number: u8,
    pub name: String,
    pub chinese: String,
    pub pinyin: String,
    pub unicode: String,
    pub description: String,
    pub judgment: JsonJudgment,
    pub image: JsonImage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonJudgment {
    pub text: String,
    pub commentary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonImage {
    pub text: String,
    pub commentary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonLineInterpretation {
    pub position: u8,
    pub text: String,
    pub comments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonReading {
    pub question: Option<String>,
    pub lines: [u8; 6],
    pub primary_hexagram: JsonHexagram,
    pub changing_lines: Vec<JsonLineInterpretation>,
    pub transformed_hexagram: Option<JsonHexagram>,
    pub upper_trigram: [String; 3],
    pub lower_trigram: [String; 3],
}

#[derive(Parser)]
#[command(name = "i-ching")]
#[command(about = "I Ching divination readings")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    /// Output format
    #[arg(short, long, default_value = "full")]
    pub format: Format,

    /// Input for reading: hexagram number (1-64), Unicode character (䷀ to ䷿), line numbers (6,7,8,9) comma separated, or changing format (32→34 or ䷟→䷡)
    #[arg(short, long)]
    pub input: Option<String>,
}

#[derive(ValueEnum, Clone)]
pub enum Format {
    Brief,
    Full,
    Json,
    Numbers,
    Motd,
}

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    let mut diviner = Diviner::new();

    let reading = if let Some(input) = cli.input {
        parse_input_and_create_reading(&mut diviner, &input)?
    } else {
        // No input provided, cast randomly using coins method
        diviner.cast_reading(None)
    };

    match cli.format {
        Format::Json => {
            let json_reading = create_json_reading(&reading)?;
            println!("{}", serde_json::to_string_pretty(&json_reading)?);
        }
        Format::Numbers => {
            println!("{:?}", reading.traditional_numbers());
        }
        Format::Brief => {
            println!("{}", format_brief(&reading)?);
        }
        Format::Full => {
            println!("{}", format_full(&reading)?);
        }
        Format::Motd => {
            println!("{}", format_motd(&reading)?);
        }
    }

    Ok(())
}

/// Parse input string and create a reading based on the input type
fn parse_input_and_create_reading(diviner: &mut Diviner, input: &str) -> Result<Reading> {
    let input = input.trim();

    // Try to parse as changing hexagram format (Unicode or numbers)
    // Supports: ䷟→䷡, ䷟->䷡, 32->34, 32→34
    if let Some(reading) = try_parse_changing_hexagram(input)? {
        return Ok(reading);
    }

    // Try to parse as hexagram number (1-64)
    if let Ok(hexagram_number) = input.parse::<u8>() {
        if hexagram_number >= 1 && hexagram_number <= 64 {
            return create_reading_from_hexagram_number(hexagram_number);
        }
    }

    // Try to parse as Unicode hexagram character
    if input.chars().count() == 1 {
        let unicode_char = input.chars().next().unwrap();
        if let Some(hexagram_number) = unicode_to_hexagram_number(unicode_char)? {
            return create_reading_from_hexagram_number(hexagram_number);
        }
    }

    // Try to parse as comma-separated line numbers (6,7,8,9)
    if input.contains(',') {
        let line_numbers: Result<Vec<u8>, _> =
            input.split(',').map(|s| s.trim().parse::<u8>()).collect();

        if let Ok(numbers) = line_numbers {
            if numbers.len() == 6 && numbers.iter().all(|&n| [6, 7, 8, 9].contains(&n)) {
                let lines_array: [u8; 6] = numbers
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Failed to convert line numbers to array"))?;
                return diviner.cast_reading_from_numbers(lines_array, None);
            }
        }
    }

    Err(anyhow::anyhow!(
        "Invalid input: '{}'. Expected hexagram number (1-64), Unicode character (䷀-䷿), changing format (32→34 or ䷟→䷡), or comma-separated line numbers (6,7,8,9)",
        input
    ))
}

/// Try to parse changing hexagram format like 32→34, 32->34, ䷟→䷡, ䷟->䷡
fn try_parse_changing_hexagram(input: &str) -> Result<Option<Reading>> {
    // Look for arrow indicators (both Unicode and ASCII)
    let separators = ["→", "->"];

    for separator in &separators {
        if let Some(arrow_pos) = input.find(separator) {
            let (from_part, to_part) = input.split_at(arrow_pos);
            let to_part = &to_part[separator.len()..];
            let from_part = from_part.trim();
            let to_part = to_part.trim();

            // Try to parse both parts as hexagram numbers
            if let (Ok(from_num), Ok(to_num)) = (from_part.parse::<u8>(), to_part.parse::<u8>()) {
                if from_num >= 1 && from_num <= 64 && to_num >= 1 && to_num <= 64 {
                    return Ok(Some(create_changing_reading_from_numbers(
                        from_num, to_num,
                    )?));
                }
            }

            // Try to parse both parts as Unicode characters
            if from_part.chars().count() == 1 && to_part.chars().count() == 1 {
                let from_char = from_part.chars().next().unwrap();
                let to_char = to_part.chars().next().unwrap();

                if let (Some(from_num), Some(to_num)) = (
                    unicode_to_hexagram_number(from_char)?,
                    unicode_to_hexagram_number(to_char)?,
                ) {
                    return Ok(Some(create_changing_reading_from_numbers(
                        from_num, to_num,
                    )?));
                }
            }
        }
    }

    Ok(None)
}

/// Convert Unicode hexagram character to hexagram number
fn unicode_to_hexagram_number(unicode_char: char) -> Result<Option<u8>> {
    let data =
        IChingData::load().map_err(|e| anyhow::anyhow!("Failed to load I Ching data: {}", e))?;

    // Search through all hexagrams to find matching Unicode character
    for i in 1..=64 {
        if let Some(hexagram) = data.get_hexagram(i) {
            if hexagram.unicode.chars().next() == Some(unicode_char) {
                return Ok(Some(i));
            }
        }
    }

    Ok(None)
}

/// Create a reading from a hexagram number by generating all young lines (no changing lines)
fn create_reading_from_hexagram_number(hexagram_number: u8) -> Result<Reading> {
    // Convert hexagram number back to binary representation
    // Hexagram numbers are 1-indexed, so subtract 1 to get 0-63 range
    let binary_value = hexagram_number - 1;

    let mut lines = [crate::core::reading::Line::new(
        crate::core::reading::Age::Young,
        crate::core::reading::Polarity::Yin,
    ); 6];

    // Convert binary representation to lines (bottom to top)
    for i in 0..6 {
        let bit = (binary_value >> i) & 1;
        lines[i] = crate::core::reading::Line::new(
            crate::core::reading::Age::Young,
            if bit == 1 {
                crate::core::reading::Polarity::Yang
            } else {
                crate::core::reading::Polarity::Yin
            },
        );
    }

    Ok(Reading::new(lines, None))
}

/// Create a reading that changes from one hexagram to another
fn create_changing_reading_from_numbers(from_hexagram: u8, to_hexagram: u8) -> Result<Reading> {
    // Convert hexagram numbers to binary representations
    let from_binary = from_hexagram - 1;
    let to_binary = to_hexagram - 1;

    let mut lines = [crate::core::reading::Line::new(
        crate::core::reading::Age::Young,
        crate::core::reading::Polarity::Yin,
    ); 6];

    // Create lines that will transform from_hexagram into to_hexagram
    for i in 0..6 {
        let from_bit = (from_binary >> i) & 1;
        let to_bit = (to_binary >> i) & 1;

        let from_polarity = if from_bit == 1 {
            crate::core::reading::Polarity::Yang
        } else {
            crate::core::reading::Polarity::Yin
        };

        let to_polarity = if to_bit == 1 {
            crate::core::reading::Polarity::Yang
        } else {
            crate::core::reading::Polarity::Yin
        };

        // If the polarity changes, make it an old line (changing)
        // If it stays the same, make it a young line (stable)
        if from_polarity != to_polarity {
            lines[i] =
                crate::core::reading::Line::new(crate::core::reading::Age::Old, from_polarity);
        } else {
            lines[i] =
                crate::core::reading::Line::new(crate::core::reading::Age::Young, from_polarity);
        }
    }

    let reading = Reading::new(lines, None);

    // Verify that our reading actually transforms correctly
    if reading.primary_hexagram() != from_hexagram {
        return Err(anyhow::anyhow!(
            "Internal error: created reading has hexagram {} but expected {}",
            reading.primary_hexagram(),
            from_hexagram
        ));
    }

    if let Some(transformed) = reading.transformed_hexagram() {
        if transformed.primary_hexagram() != to_hexagram {
            return Err(anyhow::anyhow!(
                "Internal error: transformed reading has hexagram {} but expected {}",
                transformed.primary_hexagram(),
                to_hexagram
            ));
        }
    } else if from_hexagram != to_hexagram {
        return Err(anyhow::anyhow!(
            "Internal error: reading should have changing lines but doesn't"
        ));
    }

    Ok(reading)
}

/// Create a JSON representation of a reading with full meanings
fn create_json_reading(reading: &Reading) -> Result<JsonReading> {
    let data =
        IChingData::load().map_err(|e| anyhow::anyhow!("Failed to load I Ching data: {}", e))?;

    let hexagram_number = reading.primary_hexagram();
    let hexagram = data
        .get_hexagram(hexagram_number)
        .ok_or_else(|| anyhow::anyhow!("Hexagram {} not found", hexagram_number))?;

    let primary_hexagram = JsonHexagram {
        number: hexagram.number,
        name: hexagram.name.clone(),
        chinese: hexagram.chinese.clone(),
        pinyin: hexagram.pinyin.clone(),
        unicode: hexagram.unicode.clone(),
        description: hexagram.description.clone(),
        judgment: JsonJudgment {
            text: hexagram.judgment.text.clone(),
            commentary: hexagram.judgment.commentary.clone(),
        },
        image: JsonImage {
            text: hexagram.image.text.clone(),
            commentary: hexagram.image.commentary.clone(),
        },
    };

    let changing_lines: Vec<JsonLineInterpretation> = reading
        .changing_line_positions()
        .into_iter()
        .filter_map(|line_pos| {
            data.get_line_interpretation(hexagram_number, line_pos)
                .map(|interp| JsonLineInterpretation {
                    position: line_pos,
                    text: interp.text.clone(),
                    comments: interp.comments.clone(),
                })
        })
        .collect();

    let transformed_hexagram = if let Some(transformed) = reading.transformed_hexagram() {
        let transformed_number = transformed.primary_hexagram();
        data.get_hexagram(transformed_number)
            .map(|hex| JsonHexagram {
                number: hex.number,
                name: hex.name.clone(),
                chinese: hex.chinese.clone(),
                pinyin: hex.pinyin.clone(),
                unicode: hex.unicode.clone(),
                description: hex.description.clone(),
                judgment: JsonJudgment {
                    text: hex.judgment.text.clone(),
                    commentary: hex.judgment.commentary.clone(),
                },
                image: JsonImage {
                    text: hex.image.text.clone(),
                    commentary: hex.image.commentary.clone(),
                },
            })
    } else {
        None
    };

    let polarity_to_string = |polarity| match polarity {
        crate::core::reading::Polarity::Yang => "Yang".to_string(),
        crate::core::reading::Polarity::Yin => "Yin".to_string(),
    };

    let upper_trigram = reading.upper_trigram().map(polarity_to_string);
    let lower_trigram = reading.lower_trigram().map(polarity_to_string);

    Ok(JsonReading {
        question: reading.question.clone(),
        lines: reading.traditional_numbers(),
        primary_hexagram,
        changing_lines,
        transformed_hexagram,
        upper_trigram,
        lower_trigram,
    })
}

fn format_brief(reading: &Reading) -> Result<String> {
    let data =
        IChingData::load().map_err(|e| anyhow::anyhow!("Failed to load I Ching data: {}", e))?;
    let mut result = String::new();

    if let Some(ref question) = reading.question {
        result.push_str(&format!("Q: {}\n", question));
    }

    let hexagram_number = reading.primary_hexagram();
    if let Some(hexagram) = data.get_hexagram(hexagram_number) {
        result.push_str(&format!(
            "{} {} {}",
            hexagram.unicode, hexagram_number, hexagram.name
        ));

        if reading.has_changing_lines() {
            if let Some(transformed) = reading.transformed_hexagram() {
                let transformed_number = transformed.primary_hexagram();
                if let Some(transformed_hex) = data.get_hexagram(transformed_number) {
                    result.push_str(&format!(
                        " → {} {} {}",
                        transformed_hex.unicode, transformed_number, transformed_hex.name
                    ));
                } else {
                    result.push_str(&format!(" → {} {}", transformed_number, "Unknown"));
                }
            }
            result.push_str(&format!(
                " (lines: {:?})",
                reading.changing_line_positions()
            ));
        }
    } else {
        result.push_str(&format!("Hexagram {} (Unknown)", hexagram_number));
    }

    Ok(result)
}

fn format_full(reading: &Reading) -> Result<String> {
    let data =
        IChingData::load().map_err(|e| anyhow::anyhow!("Failed to load I Ching data: {}", e))?;
    let mut result = reading.display();

    // Add traditional numbers for reference
    result.push_str(&format!(
        "\nTraditional numbers: {:?}\n",
        reading.traditional_numbers()
    ));

    // Add trigram information
    result.push_str(&format!("Upper trigram: {:?}\n", reading.upper_trigram()));
    result.push_str(&format!("Lower trigram: {:?}\n", reading.lower_trigram()));

    // Add hexagram meanings
    let hexagram_number = reading.primary_hexagram();
    if let Some(hexagram) = data.get_hexagram(hexagram_number) {
        result.push_str(&format!(
            "\n=== {} {} ===\n",
            hexagram.unicode, hexagram.name
        ));
        result.push_str(&format!(
            "Chinese: {} ({})\n",
            hexagram.chinese, hexagram.pinyin
        ));
        result.push_str(&format!("Description: {}\n", hexagram.description));

        result.push_str(&format!("\nJudgment: {}\n", hexagram.judgment.text));
        result.push_str(&format!("Commentary: {}\n", hexagram.judgment.commentary));

        result.push_str(&format!("\nImage: {}\n", hexagram.image.text));
        result.push_str(&format!(
            "Image Commentary: {}\n",
            hexagram.image.commentary
        ));

        // Add changing line interpretations
        if reading.has_changing_lines() {
            result.push_str("\n=== Changing Lines ===\n");
            for &line_pos in &reading.changing_line_positions() {
                if let Some(line_interp) = data.get_line_interpretation(hexagram_number, line_pos) {
                    result.push_str(&format!("Line {}: {}\n", line_pos, line_interp.text));
                    result.push_str(&format!("Comments: {}\n\n", line_interp.comments));
                }
            }

            // Add transformed hexagram meaning
            if let Some(transformed) = reading.transformed_hexagram() {
                let transformed_number = transformed.primary_hexagram();
                if let Some(transformed_hex) = data.get_hexagram(transformed_number) {
                    result.push_str(&format!(
                        "\n=== Transforms to {} {} ===\n",
                        transformed_hex.unicode, transformed_hex.name
                    ));
                    result.push_str(&format!(
                        "Chinese: {} ({})\n",
                        transformed_hex.chinese, transformed_hex.pinyin
                    ));
                    result.push_str(&format!("Description: {}\n", transformed_hex.description));
                    result.push_str(&format!("Judgment: {}\n", transformed_hex.judgment.text));
                }
            }
        }
    }

    Ok(result)
}

fn format_motd(reading: &Reading) -> Result<String> {
    let data =
        IChingData::load().map_err(|e| anyhow::anyhow!("Failed to load I Ching data: {}", e))?;
    let hexagram_number = reading.primary_hexagram();

    if let Some(hexagram) = data.get_hexagram(hexagram_number) {
        if reading.has_changing_lines() {
            if let Some(transformed) = reading.transformed_hexagram() {
                let transformed_number = transformed.primary_hexagram();
                if let Some(transformed_hex) = data.get_hexagram(transformed_number) {
                    Ok(format!(
                        "{}→{} {} {} CHANGING INTO {} {}",
                        hexagram.unicode,
                        transformed_hex.unicode,
                        hexagram_number,
                        hexagram.name.to_uppercase(),
                        transformed_number,
                        transformed_hex.name.to_uppercase()
                    ))
                } else {
                    Ok(format!(
                        "{}→䷜ {} {} CHANGING INTO {} UNKNOWN",
                        hexagram.unicode,
                        hexagram_number,
                        hexagram.name.to_uppercase(),
                        transformed_number
                    ))
                }
            } else {
                // This shouldn't happen if has_changing_lines() is true, but just in case
                Ok(format!(
                    "{} {} {}",
                    hexagram.unicode,
                    hexagram_number,
                    hexagram.name.to_uppercase()
                ))
            }
        } else {
            Ok(format!(
                "{} {} {}",
                hexagram.unicode,
                hexagram_number,
                hexagram.name.to_uppercase()
            ))
        }
    } else {
        Ok(format!("䷜ {} UNKNOWN", hexagram_number))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_brief() {
        let diviner = Diviner::new();
        let reading = diviner
            .cast_reading_from_numbers([7, 8, 9, 6, 7, 8], Some("Test question".to_string()))
            .unwrap();

        let brief = format_brief(&reading).unwrap();
        println!("Brief output: '{}'", brief);
        assert!(brief.contains("Q: Test question"));
        // Just check that it has some content - the specific format may vary
        assert!(!brief.is_empty());
    }

    #[test]
    fn test_format_full() {
        let diviner = Diviner::new();
        let reading = diviner
            .cast_reading_from_numbers([7, 8, 7, 8, 7, 8], Some("Test question".to_string()))
            .unwrap();

        let full = format_full(&reading).unwrap();
        assert!(full.contains("Question: Test question"));
        assert!(full.contains("Traditional numbers"));
        assert!(full.contains("Upper trigram"));
        assert!(full.contains("Lower trigram"));
    }

    #[test]
    fn test_parse_hexagram_number() {
        let mut diviner = Diviner::new();
        let reading = parse_input_and_create_reading(&mut diviner, "1").unwrap();
        assert_eq!(reading.primary_hexagram(), 1);
    }

    #[test]
    fn test_parse_line_numbers() {
        let mut diviner = Diviner::new();
        let reading = parse_input_and_create_reading(&mut diviner, "7,8,9,6,7,8").unwrap();
        assert_eq!(reading.traditional_numbers(), [7, 8, 9, 6, 7, 8]);
    }

    #[test]
    fn test_parse_unicode_character() {
        let mut diviner = Diviner::new();
        let reading = parse_input_and_create_reading(&mut diviner, "䷀").unwrap();
        assert_eq!(reading.primary_hexagram(), 1);
    }

    #[test]
    fn test_invalid_input() {
        let mut diviner = Diviner::new();
        assert!(parse_input_and_create_reading(&mut diviner, "65").is_err());
        assert!(parse_input_and_create_reading(&mut diviner, "7,8,5,6,7,8").is_err());
        assert!(parse_input_and_create_reading(&mut diviner, "invalid").is_err());
    }

    #[test]
    fn test_parse_changing_hexagram_numbers() {
        let mut diviner = Diviner::new();
        let reading = parse_input_and_create_reading(&mut diviner, "32→34").unwrap();
        assert_eq!(reading.primary_hexagram(), 32);
        assert!(reading.has_changing_lines());
        if let Some(transformed) = reading.transformed_hexagram() {
            assert_eq!(transformed.primary_hexagram(), 34);
        } else {
            panic!("Expected transformed hexagram");
        }
    }

    #[test]
    fn test_parse_changing_hexagram_ascii_arrow() {
        let mut diviner = Diviner::new();
        let reading = parse_input_and_create_reading(&mut diviner, "1->2").unwrap();
        assert_eq!(reading.primary_hexagram(), 1);
        assert!(reading.has_changing_lines());
        if let Some(transformed) = reading.transformed_hexagram() {
            assert_eq!(transformed.primary_hexagram(), 2);
        } else {
            panic!("Expected transformed hexagram");
        }
    }

    #[test]
    fn test_parse_changing_hexagram_unicode() {
        let mut diviner = Diviner::new();
        let reading = parse_input_and_create_reading(&mut diviner, "䷀→䷁").unwrap();
        assert_eq!(reading.primary_hexagram(), 1);
        assert!(reading.has_changing_lines());
        if let Some(transformed) = reading.transformed_hexagram() {
            assert_eq!(transformed.primary_hexagram(), 2);
        } else {
            panic!("Expected transformed hexagram");
        }
    }
}
