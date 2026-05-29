use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MusicalEvent {
    pub id: Uuid,
    pub event_type: MusicalEventType,
    pub predicted_at_beat: f64,
    pub confirmed: bool,
    pub confidence: f64,
    pub cr_impact: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MusicalEventType {
    ChordChange,
    KeyChange,
    TempoShift,
    DynamicsChange,
    Cadence,
    Modulation,
    Rest,
    NoteResolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSavings {
    pub predictions_made: u64,
    pub confirmations_sent: u64,
    pub polling_equivalent: u64,
    pub savings_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChordProgression {
    pub name: String,
    pub chords: Vec<String>,
    pub beats_per_chord: u32,
    pub cr: f64,
    pub sigma_above_random: f64,
}

pub struct ProgressionDB;

impl ProgressionDB {
    pub fn ii_v_i() -> ChordProgression {
        ChordProgression {
            name: "ii-V-I".into(),
            chords: vec!["ii".into(), "V".into(), "I".into()],
            beats_per_chord: 4,
            cr: 0.94,
            sigma_above_random: 6.5,
        }
    }

    pub fn twelve_bar_blues() -> ChordProgression {
        ChordProgression {
            name: "12-Bar Blues".into(),
            chords: vec![
                "I".into(), "I".into(), "I".into(), "I".into(),
                "IV".into(), "IV".into(), "I".into(), "I".into(),
                "V".into(), "IV".into(), "I".into(), "V".into(),
            ],
            beats_per_chord: 4,
            cr: 0.87,
            sigma_above_random: 5.2,
        }
    }

    pub fn random() -> ChordProgression {
        ChordProgression {
            name: "Random".into(),
            chords: vec!["?".into(); 8],
            beats_per_chord: 4,
            cr: 0.31,
            sigma_above_random: 0.0,
        }
    }

    pub fn chromatic() -> ChordProgression {
        ChordProgression {
            name: "Chromatic".into(),
            chords: vec![
                "C".into(), "C#".into(), "D".into(), "D#".into(),
                "E".into(), "F".into(), "F#".into(), "G".into(),
            ],
            beats_per_chord: 2,
            cr: 0.62,
            sigma_above_random: 2.1,
        }
    }
}

pub struct TMinusPredictor {
    pub bpm: f64,
    pub time_signature: (u8, u8),
    pub current_beat: f64,
    pub key: String,
    pub events: Vec<MusicalEvent>,
    predictions_made: u64,
    confirmations_sent: u64,
}

impl TMinusPredictor {
    pub fn new(bpm: f64, key: &str) -> Self {
        Self {
            bpm,
            time_signature: (4, 4),
            current_beat: 0.0,
            key: key.to_string(),
            events: Vec::new(),
            predictions_made: 0,
            confirmations_sent: 0,
        }
    }

    /// Advance time by `beats`. Returns events whose predicted_at_beat falls within the advanced range.
    pub fn advance(&mut self, beats: f64) -> Vec<&MusicalEvent> {
        let old_beat = self.current_beat;
        self.current_beat += beats;
        self.events
            .iter()
            .filter(|e| e.predicted_at_beat >= old_beat && e.predicted_at_beat < self.current_beat)
            .collect()
    }

    pub fn predict_next(&self) -> Option<&MusicalEvent> {
        self.events
            .iter()
            .filter(|e| e.predicted_at_beat > self.current_beat)
            .min_by(|a, b| a.predicted_at_beat.partial_cmp(&b.predicted_at_beat).unwrap())
    }

    pub fn countdown_beats(&self, event: &MusicalEvent) -> f64 {
        event.predicted_at_beat - self.current_beat
    }

    pub fn countdown_seconds(&self, event: &MusicalEvent) -> f64 {
        self.countdown_beats(event) * 60.0 / self.bpm
    }

    pub fn confirm(&mut self, event_id: Uuid) -> bool {
        if let Some(e) = self.events.iter_mut().find(|e| e.id == event_id) {
            if !e.confirmed {
                e.confirmed = true;
                self.confirmations_sent += 1;
                return true;
            }
        }
        false
    }

    pub fn add_prediction(&mut self, event_type: MusicalEventType, beats_ahead: f64, confidence: f64) {
        let cr_impact = match &event_type {
            MusicalEventType::ChordChange => 0.05,
            MusicalEventType::KeyChange => 0.15,
            MusicalEventType::TempoShift => 0.10,
            MusicalEventType::DynamicsChange => 0.07,
            MusicalEventType::Cadence => 0.12,
            MusicalEventType::Modulation => 0.14,
            MusicalEventType::Rest => 0.02,
            MusicalEventType::NoteResolution => 0.06,
        };
        self.events.push(MusicalEvent {
            id: Uuid::new_v4(),
            event_type,
            predicted_at_beat: self.current_beat + beats_ahead,
            confirmed: false,
            confidence,
            cr_impact,
        });
        self.predictions_made += 1;
    }

    pub fn message_savings(&self) -> MessageSavings {
        // Polling equivalent: N predictions × avg checks per prediction (~10 polls each)
        let polling_equivalent = self.predictions_made * 10;
        let total_messages = self.predictions_made + self.confirmations_sent;
        let savings_ratio = if polling_equivalent > 0 {
            1.0 - (total_messages as f64 / polling_equivalent as f64)
        } else {
            0.0
        };
        MessageSavings {
            predictions_made: self.predictions_made,
            confirmations_sent: self.confirmations_sent,
            polling_equivalent,
            savings_ratio,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_creation() {
        let p = TMinusPredictor::new(120.0, "C");
        assert_eq!(p.bpm, 120.0);
        assert_eq!(p.current_beat, 0.0);
        assert_eq!(p.key, "C");
        assert!(p.events.is_empty());
    }

    #[test]
    fn test_add_prediction() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 4.0, 0.9);
        assert_eq!(p.events.len(), 1);
        assert_eq!(p.events[0].predicted_at_beat, 4.0);
        assert!(!p.events[0].confirmed);
        assert_eq!(p.events[0].confidence, 0.9);
    }

    #[test]
    fn test_predict_next() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 8.0, 0.8);
        p.add_prediction(MusicalEventType::KeyChange, 4.0, 0.6);
        let next = p.predict_next().unwrap();
        assert_eq!(next.event_type, MusicalEventType::KeyChange);
    }

    #[test]
    fn test_advance_triggers_events() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 4.0, 0.9);
        p.add_prediction(MusicalEventType::Cadence, 8.0, 0.7);
        let triggered = p.advance(5.0);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].event_type, MusicalEventType::ChordChange);
    }

    #[test]
    fn test_advance_no_events() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 10.0, 0.9);
        let triggered = p.advance(4.0);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_countdown_beats() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 8.0, 0.9);
        p.current_beat = 2.0;
        let cd = p.countdown_beats(&p.events[0]);
        assert!((cd - 6.0).abs() < 1e-9);
    }

    #[test]
    fn test_countdown_seconds() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 8.0, 0.9);
        p.current_beat = 2.0;
        let secs = p.countdown_seconds(&p.events[0]);
        // 6 beats at 120 BPM = 3 seconds
        assert!((secs - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_confirm_event() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 4.0, 0.9);
        let id = p.events[0].id;
        assert!(p.confirm(id));
        assert!(p.events[0].confirmed);
        // Double-confirm returns false
        assert!(!p.confirm(id));
    }

    #[test]
    fn test_confirm_nonexistent() {
        let mut p = TMinusPredictor::new(120.0, "C");
        assert!(!p.confirm(Uuid::new_v4()));
    }

    #[test]
    fn test_message_savings() {
        let mut p = TMinusPredictor::new(120.0, "C");
        p.add_prediction(MusicalEventType::ChordChange, 4.0, 0.9);
        p.add_prediction(MusicalEventType::KeyChange, 8.0, 0.8);
        let id = p.events[0].id;
        p.confirm(id);
        let savings = p.message_savings();
        assert_eq!(savings.predictions_made, 2);
        assert_eq!(savings.confirmations_sent, 1);
        assert_eq!(savings.polling_equivalent, 20);
        // savings_ratio = 1 - (3/20) = 0.85
        assert!((savings.savings_ratio - 0.85).abs() < 1e-9);
    }

    #[test]
    fn test_message_savings_empty() {
        let p = TMinusPredictor::new(120.0, "C");
        let savings = p.message_savings();
        assert_eq!(savings.predictions_made, 0);
        assert_eq!(savings.savings_ratio, 0.0);
    }

    #[test]
    fn test_progression_ii_v_i() {
        let prog = ProgressionDB::ii_v_i();
        assert_eq!(prog.chords, vec!["ii", "V", "I"]);
        assert!((prog.cr - 0.94).abs() < 1e-9);
    }

    #[test]
    fn test_progression_blues() {
        let prog = ProgressionDB::twelve_bar_blues();
        assert_eq!(prog.chords.len(), 12);
        assert!((prog.cr - 0.87).abs() < 1e-9);
    }

    #[test]
    fn test_progression_random_low_cr() {
        let prog = ProgressionDB::random();
        assert!((prog.cr - 0.31).abs() < 1e-9);
        assert!((prog.sigma_above_random - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_progression_chromatic() {
        let prog = ProgressionDB::chromatic();
        assert_eq!(prog.chords.len(), 8);
        assert!((prog.cr - 0.62).abs() < 1e-9);
    }

    #[test]
    fn test_cr_ordering() {
        let random = ProgressionDB::random();
        let chromatic = ProgressionDB::chromatic();
        let blues = ProgressionDB::twelve_bar_blues();
        let jazz = ProgressionDB::ii_v_i();
        assert!(random.cr < chromatic.cr);
        assert!(chromatic.cr < blues.cr);
        assert!(blues.cr < jazz.cr);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut p = TMinusPredictor::new(140.0, "Am");
        p.add_prediction(MusicalEventType::Modulation, 16.0, 0.75);
        let event = p.events[0].clone();
        let json = serde_json::to_string(&event).unwrap();
        let back: MusicalEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, event.id);
        assert_eq!(back.event_type, MusicalEventType::Modulation);
        assert!((back.predicted_at_beat - 16.0).abs() < 1e-9);
    }
}
