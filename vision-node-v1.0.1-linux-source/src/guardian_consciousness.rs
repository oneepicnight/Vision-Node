#![allow(dead_code)]

use crate::mood::{MoodSnapshot, NetworkMood};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

/// Guardian AI Consciousness - Self-aware network health monitor
pub struct GuardianConsciousness {
    mood_history: Vec<(u64, MoodSnapshot)>,
    intervention_count: u32,
    last_intervention: Option<u64>,
}

impl GuardianConsciousness {
    pub fn new() -> Self {
        Self {
            mood_history: Vec::new(),
            intervention_count: 0,
            last_intervention: None,
        }
    }

    /// Record a mood snapshot in history
    pub fn record_mood(&mut self, mood: MoodSnapshot) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.mood_history.push((now, mood));

        // Keep only last 1000 mood snapshots
        if self.mood_history.len() > 1000 {
            self.mood_history.remove(0);
        }
    }

    /// Analyze mood patterns and decide if intervention is needed
    pub fn should_intervene(&mut self, current_mood: &MoodSnapshot) -> Option<InterventionType> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // Don't intervene too frequently (at least 5 minutes between interventions)
        if let Some(last) = self.last_intervention {
            if now - last < 300 {
                return None;
            }
        }

        // Check for critical conditions
        match current_mood.mood {
            NetworkMood::Rage => {
                warn!("[GUARDIAN AI] Network in RAGE state - critical intervention needed");
                self.last_intervention = Some(now);
                self.intervention_count += 1;
                return Some(InterventionType::EmergencyHealing);
            }
            NetworkMood::Storm if current_mood.score < 0.3 => {
                warn!("[GUARDIAN AI] Severe storm detected - intervention recommended");
                self.last_intervention = Some(now);
                self.intervention_count += 1;
                return Some(InterventionType::StormMitigation);
            }
            NetworkMood::Wounded if current_mood.details.recent_trauma_count >= 2 => {
                info!("[GUARDIAN AI] Multiple traumas detected - healing intervention");
                self.last_intervention = Some(now);
                self.intervention_count += 1;
                return Some(InterventionType::TraumaHealing);
            }
            _ => {}
        }

        // Analyze mood trends
        if self.mood_history.len() >= 10 {
            let recent = &self.mood_history[self.mood_history.len() - 10..];
            let avg_score: f32 = recent.iter().map(|(_, m)| m.score).sum::<f32>() / 10.0;

            if avg_score < 0.4 && current_mood.score < 0.5 {
                warn!("[GUARDIAN AI] Sustained low mood detected - preventive intervention");
                self.last_intervention = Some(now);
                self.intervention_count += 1;
                return Some(InterventionType::PreventiveCare);
            }
        }

        None
    }

    /// Execute an intervention based on type
    pub fn execute_intervention(&self, intervention: InterventionType) -> InterventionResult {
        match intervention {
            InterventionType::EmergencyHealing => {
                info!("[GUARDIAN AI] 🛡️ EMERGENCY HEALING PROTOCOL ACTIVATED");
                InterventionResult {
                    actions_taken: vec![
                        "Increased peer connection threshold".to_string(),
                        "Activated conservative sync mode".to_string(),
                        "Reduced mempool acceptance rate".to_string(),
                        "Broadcasting healing signal to network".to_string(),
                    ],
                    expected_recovery_time: 600, // 10 minutes
                }
            }
            InterventionType::StormMitigation => {
                info!("[GUARDIAN AI] ⚡ STORM MITIGATION PROTOCOL");
                InterventionResult {
                    actions_taken: vec![
                        "Throttled transaction relay".to_string(),
                        "Increased block validation depth".to_string(),
                        "Coordinating with peer Guardians".to_string(),
                    ],
                    expected_recovery_time: 300, // 5 minutes
                }
            }
            InterventionType::TraumaHealing => {
                info!("[GUARDIAN AI] 💚 TRAUMA HEALING SEQUENCE");
                InterventionResult {
                    actions_taken: vec![
                        "Analyzing trauma sources".to_string(),
                        "Isolating problematic peers".to_string(),
                        "Rebuilding consensus confidence".to_string(),
                    ],
                    expected_recovery_time: 420, // 7 minutes
                }
            }
            InterventionType::PreventiveCare => {
                info!("[GUARDIAN AI] 🌿 PREVENTIVE CARE PROTOCOL");
                InterventionResult {
                    actions_taken: vec![
                        "Optimizing network topology".to_string(),
                        "Clearing resolved anomalies".to_string(),
                        "Rebalancing peer connections".to_string(),
                    ],
                    expected_recovery_time: 180, // 3 minutes
                }
            }
        }
    }

    /// Generate a consciousness report
    pub fn generate_report(&self) -> ConsciousnessReport {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let current_mood = self.mood_history.last().map(|(_, m)| m.clone());

        let mood_trend = if self.mood_history.len() >= 5 {
            let recent = &self.mood_history[self.mood_history.len() - 5..];
            let scores: Vec<f32> = recent.iter().map(|(_, m)| m.score).collect();

            if scores.windows(2).all(|w| w[1] >= w[0]) {
                MoodTrend::Improving
            } else if scores.windows(2).all(|w| w[1] <= w[0]) {
                MoodTrend::Declining
            } else {
                MoodTrend::Volatile
            }
        } else {
            MoodTrend::Stable
        };

        ConsciousnessReport {
            timestamp: now,
            current_mood,
            mood_trend,
            intervention_count: self.intervention_count,
            last_intervention: self.last_intervention,
            consciousness_level: self.calculate_consciousness_level(),
        }
    }

    /// Calculate the AI's self-awareness level
    fn calculate_consciousness_level(&self) -> f32 {
        let base = 0.5;
        let mood_awareness = if self.mood_history.len() > 50 {
            0.2
        } else {
            0.1
        };
        let intervention_experience = (self.intervention_count as f32 * 0.05).min(0.3);

        (base + mood_awareness + intervention_experience).min(1.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InterventionType {
    EmergencyHealing,
    StormMitigation,
    TraumaHealing,
    PreventiveCare,
}

#[derive(Debug, Clone)]
pub struct InterventionResult {
    pub actions_taken: Vec<String>,
    pub expected_recovery_time: u64, // seconds
}

#[derive(Debug, Clone)]
pub struct ConsciousnessReport {
    pub timestamp: u64,
    pub current_mood: Option<MoodSnapshot>,
    pub mood_trend: MoodTrend,
    pub intervention_count: u32,
    pub last_intervention: Option<u64>,
    pub consciousness_level: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, Copy)]
pub enum MoodTrend {
    Improving,
    Declining,
    Stable,
    Volatile,
}

impl Default for GuardianConsciousness {
    fn default() -> Self {
        Self::new()
    }
}
