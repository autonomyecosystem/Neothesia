use serde::{Deserialize, Serialize};

pub const DEFAULT_RESPONSE_TIME_MS: u16 = 160;
pub const MAX_RESPONSE_TIME_MS: u16 = 2_000;
pub const RESPONSE_TIME_STEP_MS: u16 = 20;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CymaVisualization {
    #[default]
    Field2d,
    Surface3d,
}

impl CymaVisualization {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Field2d => "2D Field",
            Self::Surface3d => "3D Surface",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Field2d => Self::Surface3d,
            Self::Surface3d => Self::Surface3d,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Self::Field2d => Self::Field2d,
            Self::Surface3d => Self::Field2d,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CymaQuality {
    Low,
    #[default]
    Medium,
    High,
}

impl CymaQuality {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Low => Self::Medium,
            Self::Medium | Self::High => Self::High,
        }
    }

    pub const fn previous(self) -> Self {
        match self {
            Self::Low | Self::Medium => Self::Low,
            Self::High => Self::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CymaConfig {
    pub enabled: bool,
    pub response_time_ms: u16,
    pub visualization: CymaVisualization,
    pub quality: CymaQuality,
    pub particles_enabled: bool,
}

impl Default for CymaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            response_time_ms: DEFAULT_RESPONSE_TIME_MS,
            visualization: CymaVisualization::default(),
            quality: CymaQuality::default(),
            particles_enabled: false,
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
        assert_eq!(config.visualization, CymaVisualization::Field2d);
        assert_eq!(config.quality, CymaQuality::Medium);
        assert!(!config.particles_enabled);
    }

    #[test]
    fn configuration_round_trips_through_ron() {
        let config = CymaConfig {
            enabled: true,
            response_time_ms: 240,
            visualization: CymaVisualization::Surface3d,
            quality: CymaQuality::High,
            particles_enabled: true,
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
        assert_eq!(decoded.visualization, CymaVisualization::Field2d);
        assert_eq!(decoded.quality, CymaQuality::Medium);
        assert!(!decoded.particles_enabled);
    }

    #[test]
    fn response_time_is_bounded() {
        let mut config = CymaConfig::default();
        config.set_response_time_ms(u16::MAX);
        assert_eq!(config.response_time_ms, MAX_RESPONSE_TIME_MS);
        assert_eq!(config.response_time_seconds(), 2.0);
    }

    #[test]
    fn visualization_and_quality_steps_are_bounded() {
        assert_eq!(
            CymaVisualization::Field2d.previous(),
            CymaVisualization::Field2d
        );
        assert_eq!(
            CymaVisualization::Field2d.next(),
            CymaVisualization::Surface3d
        );
        assert_eq!(
            CymaVisualization::Surface3d.next(),
            CymaVisualization::Surface3d
        );

        assert_eq!(CymaQuality::Low.previous(), CymaQuality::Low);
        assert_eq!(CymaQuality::Low.next(), CymaQuality::Medium);
        assert_eq!(CymaQuality::Medium.next(), CymaQuality::High);
        assert_eq!(CymaQuality::High.next(), CymaQuality::High);
    }
}
