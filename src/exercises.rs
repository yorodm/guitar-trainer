use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::sync::OnceLock;

static EXERCISES: OnceLock<Vec<Exercise>> = OnceLock::new();

pub fn set_exercises(data: Vec<Exercise>) {
    validate_exercise_ids(&data);
    let _ = EXERCISES.set(data);
}

pub fn all_exercises() -> &'static Vec<Exercise> {
    EXERCISES.get_or_init(|| {
        let exercises: Vec<Exercise> = serde_json::from_str(include_str!("../src-tauri/resources/exercises.json"))
            .expect("embedded exercises.json is valid");
        validate_exercise_ids(&exercises);
        exercises
    })
}

fn validate_exercise_ids(exercises: &[Exercise]) {
    for (i, ex) in exercises.iter().enumerate() {
        let expected = (i + 1) as u32;
        assert!(
            ex.id == expected,
            "Exercise '{}' has id {} but expected {} (IDs must be contiguous starting at 1)",
            ex.name, ex.id, expected
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Category {
    Warmup,
    Coordination,
    EarTraining,
    Stamina,
    Rhythm,
    Arpeggios,
    Modes,
    CrossPicking,
    MajorScales,
    Other,
}

impl Category {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Warmup => "Warmup",
            Self::Coordination => "Coordination",
            Self::EarTraining => "Ear Training",
            Self::Stamina => "Stamina",
            Self::Rhythm => "Rhythm",
            Self::Arpeggios => "Arpeggios",
            Self::Modes => "Modes",
            Self::CrossPicking => "Cross Picking",
            Self::MajorScales => "Major Scales",
            Self::Other => "Other",
        }
    }

    pub const fn all() -> &'static [Self] {
        &[
            Self::Warmup,
            Self::Coordination,
            Self::EarTraining,
            Self::Stamina,
            Self::Rhythm,
            Self::Arpeggios,
            Self::Modes,
            Self::CrossPicking,
            Self::MajorScales,
            Self::Other,
        ]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum PickingDirection {
    Down = 0,
    Up = 1,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Finger {
    Index = 0,
    Middle = 1,
    Ring = 2,
    Pinky = 3,
    Open = 4,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum NoteDuration {
    Whole = 0,
    Half = 1,
    Quarter = 2,
    Eighth = 3,
    Sixteenth = 4,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Note {
    pub string: u8,
    pub fret: u8,
    pub duration: NoteDuration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exercise {
    pub id: u32,
    pub name: String,
    pub category: Category,
    pub description: String,
    pub default_bpm: u16,
    pub min_bpm: u16,
    pub max_bpm: u16,
    pub default_duration_secs: u32,
    pub notes: Vec<Note>,
    pub picking: Vec<PickingDirection>,
    pub fingering: Vec<Finger>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_count() {
        assert_eq!(Category::all().len(), 10);
    }

    #[test]
    fn category_names() {
        assert_eq!(Category::Warmup.name(), "Warmup");
        assert_eq!(Category::EarTraining.name(), "Ear Training");
        assert_eq!(Category::MajorScales.name(), "Major Scales");
    }

    #[test]
    fn exercises_have_unique_ids() {
        let exercises = all_exercises();
        let mut ids: Vec<u32> = exercises.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), exercises.len());
    }

    #[test]
    fn exercises_have_valid_string_numbers() {
        for ex in all_exercises() {
            for note in &ex.notes {
                assert!(note.string >= 1 && note.string <= 6);
            }
        }
    }

    #[test]
    fn picking_is_alternating() {
        for ex in all_exercises() {
            for pair in ex.picking.windows(2) {
                assert_ne!(pair[0], pair[1]);
            }
        }
    }

    #[test]
    fn json_is_valid_and_matches_struct() {
        let json = include_str!("../src-tauri/resources/exercises.json");
        let parsed: Vec<Exercise> = serde_json::from_str(json).unwrap();
        assert!(!parsed.is_empty());
        assert_eq!(parsed.len(), all_exercises().len());
    }
}
