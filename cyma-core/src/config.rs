use serde::{Deserialize, Serialize};

pub const DEFAULT_RESPONSE_TIME_MS: u16 = 160;

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
        f32::from(self.response_time_ms) / 1_000.0
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
}
