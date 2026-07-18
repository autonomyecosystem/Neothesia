use serde::{Deserialize, Serialize};

pub const DEFAULT_RESPONSE_TIME_MS: u16 = 160;
pub const MAX_RESPONSE_TIME_MS: u16 = 2_000;
pub const RESPONSE_TIME_STEP_MS: u16 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CymaConfig {
    pub enabled: bool,
    pub response_time_ms: u16,
}

impl Default for CymaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            response_time_ms: DEFAULT_RESPONSE_TIME_MS,
        }
    }
}

impl CymaConfig {
    pub fn response_time_seconds(self) -> f32 {
        f32::from(self.response_time_ms.min(MAX_RESPONSE_TIME_MS)) / 1_000.0
    }

    pub fn set_response_time_ms(&mut self, response_time_ms: u16) {
        self.response_time_ms = response_time_ms.min(MAX_RESPONSE_TIME_MS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_backwards_compatible() {
        let config = CymaConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.response_time_ms, DEFAULT_RESPONSE_TIME_MS);
    }

    #[test]
    fn configuration_round_trips_through_ron() {
        let config = CymaConfig {
            enabled: true,
            response_time_ms: 240,
        };
        let encoded = ron::to_string(&config).unwrap();
        let decoded: CymaConfig = ron::from_str(&encoded).unwrap();
        assert_eq!(decoded, config);
    }

    #[test]
    fn missing_fields_use_defaults() {
        let decoded: CymaConfig = ron::from_str("(enabled:true)").unwrap();
        assert!(decoded.enabled);
        assert_eq!(decoded.response_time_ms, DEFAULT_RESPONSE_TIME_MS);
    }

    #[test]
    fn response_time_is_bounded() {
        let mut config = CymaConfig::default();
        config.set_response_time_ms(u16::MAX);
        assert_eq!(config.response_time_ms, MAX_RESPONSE_TIME_MS);
        assert_eq!(config.response_time_seconds(), 2.0);
    }
}
