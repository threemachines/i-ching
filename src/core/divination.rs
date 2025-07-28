use crate::core::reading::{Age, Line, Polarity, Reading};
use rand::Rng;

pub struct Diviner {
    rng: rand::rngs::ThreadRng,
}

impl Diviner {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }

    /// Cast a complete reading using the three coins method
    pub fn cast_reading(&mut self, question: Option<String>) -> Reading {
        let lines = [
            self.cast_line(),
            self.cast_line(),
            self.cast_line(),
            self.cast_line(),
            self.cast_line(),
            self.cast_line(),
        ];

        Reading::new(lines, question)
    }

    /// Convert a traditional line number (6-9) to a Line
    ///
    /// Traditional interpretation:
    /// - 6 (2+2+2): Old Yin (changing) - probability 1/8
    /// - 7 (2+2+3): Young Yang - probability 3/8
    /// - 8 (2+3+3): Young Yin - probability 3/8
    /// - 9 (3+3+3): Old Yang (changing) - probability 1/8
    fn number_to_line(number: u8) -> Line {
        match number {
            6 => Line::new(Age::Old, Polarity::Yin),    // Old Yin
            7 => Line::new(Age::Young, Polarity::Yang), // Young Yang
            8 => Line::new(Age::Young, Polarity::Yin),  // Young Yin
            9 => Line::new(Age::Old, Polarity::Yang),   // Old Yang
            _ => panic!("Invalid line number: {}. Must be 6, 7, 8, or 9", number),
        }
    }

    /// Cast a single line using three coins
    ///
    /// Each coin contributes 2 (tails) or 3 (heads), giving totals of 6-9.
    /// See `number_to_line` for probability details.
    fn cast_line(&mut self) -> Line {
        let coin_sum: u8 = (0..3)
            .map(|_| if self.rng.gen_bool(0.5) { 3 } else { 2 })
            .sum();

        Self::number_to_line(coin_sum)
    }

    /// Cast a reading from specific line numbers (6, 7, 8, 9)
    pub fn cast_reading_from_numbers(
        &self,
        numbers: [u8; 6],
        question: Option<String>,
    ) -> Result<Reading, anyhow::Error> {
        let mut lines = [Line::new(Age::Young, Polarity::Yang); 6];

        for (i, &num) in numbers.iter().enumerate() {
            lines[i] = Line::from_traditional_number(num)?;
        }

        Ok(Reading::new(lines, question))
    }

    /// Cast a reading using specific line numbers (for testing)
    #[cfg(test)]
    pub fn cast_reading_with_numbers(
        &mut self,
        numbers: [u8; 6],
        question: Option<String>,
    ) -> Reading {
        let lines = numbers.map(Self::number_to_line);
        Reading::new(lines, question)
    }
}

impl Default for Diviner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_number_interpretation() {
        let mut diviner = Diviner::new();

        // Test specific line numbers directly
        let numbers = [6, 7, 8, 9, 6, 8];

        let reading = diviner.cast_reading_with_numbers(numbers, Some("Test question".to_string()));

        assert_eq!(reading.traditional_numbers(), [6, 7, 8, 9, 6, 8]);
        assert!(reading.has_changing_lines());
        assert_eq!(reading.changing_line_positions(), vec![1, 4, 5]); // Lines 1, 4, 5 are changing (6, 9, 6)
    }

    #[test]
    fn test_cast_from_numbers() {
        let diviner = Diviner::new();
        let numbers = [7, 8, 9, 6, 7, 8];

        let reading = diviner
            .cast_reading_from_numbers(numbers, Some("Test from numbers".to_string()))
            .unwrap();

        assert_eq!(reading.traditional_numbers(), numbers);
        assert!(reading.has_changing_lines());
        assert_eq!(reading.changing_line_positions(), vec![3, 4]); // Lines 3, 4 are changing (9, 6)
    }

    #[test]
    fn test_invalid_numbers() {
        let diviner = Diviner::new();
        let invalid_numbers = [7, 8, 5, 6, 7, 8]; // 5 is invalid

        let result = diviner.cast_reading_from_numbers(invalid_numbers, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid line number: 5"));
    }

    #[test]
    fn test_random_casting() {
        let mut diviner = Diviner::new();

        // Just test that it doesn't panic and produces valid results
        for _ in 0..10 {
            let reading = diviner.cast_reading(Some("Random test".to_string()));

            // Verify all traditional numbers are valid
            for &num in &reading.traditional_numbers() {
                assert!([6, 7, 8, 9].contains(&num));
            }

            // Verify hexagram number is in valid range
            let hexagram = reading.primary_hexagram();
            assert!(hexagram >= 1 && hexagram <= 64);
        }
    }
}
