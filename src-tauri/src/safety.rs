//! Turns raw memory readings into a green / yellow / red judgement.
//!
//! Pure functions only — no syscalls — so the rules are testable as a truth table.
//! The kernel's pressure signal is authoritative when it reports warning or critical;
//! headroom and swap are heuristics layered on top, and the worst signal wins.

use serde::Serialize;

use crate::sysmem::Pressure;

const GIB: u64 = 1024 * 1024 * 1024;

/// Below this much projected headroom, macOS and the user's other applications are
/// being squeezed. Calibrated for a machine also running an editor, a browser and a
/// coding agent — roughly 9-10 GB of resident non-model work.
pub const HEADROOM_RED: i64 = 2 * GIB as i64;
pub const HEADROOM_YELLOW: i64 = 4 * GIB as i64;
pub const SWAP_YELLOW: u64 = 2 * GIB;
pub const SWAP_RED: u64 = 6 * GIB;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SafetyState {
    Unknown,
    Green,
    Yellow,
    Red,
}

impl SafetyState {
    fn rank(self) -> u8 {
        match self {
            SafetyState::Unknown => 0,
            SafetyState::Green => 1,
            SafetyState::Yellow => 2,
            SafetyState::Red => 3,
        }
    }

    fn worst(self, other: SafetyState) -> SafetyState {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Inputs {
    pub installed: Option<u64>,
    pub used: Option<u64>,
    pub swap_used: Option<u64>,
    pub pressure: Pressure,
    /// Memory already attributable to a model that this launch would replace, since only
    /// one model runs at a time. Subtracted before adding the prediction.
    pub running_model_bytes: Option<u64>,
    /// Predicted total for a launch under consideration. `None` means "assess the
    /// machine as it stands" rather than a hypothetical launch.
    pub predicted_total: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    pub state: SafetyState,
    pub projected_used_bytes: Option<u64>,
    pub headroom_bytes: Option<i64>,
    pub reasons: Vec<String>,
}

fn gb(bytes: i64) -> String {
    format!("{:.1} GB", bytes as f64 / GIB as f64)
}

pub fn assess(inputs: Inputs) -> Assessment {
    let projected = inputs.used.map(|used| {
        let without_current = used.saturating_sub(inputs.running_model_bytes.unwrap_or(0));
        without_current + inputs.predicted_total.unwrap_or(0)
    });

    let headroom = match (inputs.installed, projected) {
        (Some(installed), Some(projected)) => Some(installed as i64 - projected as i64),
        _ => None,
    };

    let mut state = SafetyState::Unknown;
    let mut reasons = Vec::new();

    match inputs.pressure {
        Pressure::Critical => {
            state = state.worst(SafetyState::Red);
            reasons.push("macOS reports critical memory pressure".to_string());
        }
        Pressure::Warning => {
            state = state.worst(SafetyState::Yellow);
            reasons.push("macOS reports elevated memory pressure".to_string());
        }
        Pressure::Normal => state = state.worst(SafetyState::Green),
        Pressure::Unknown => {}
    }

    if let Some(headroom) = headroom {
        if headroom < HEADROOM_RED {
            state = state.worst(SafetyState::Red);
            reasons.push(format!(
                "only {} would be left for macOS and everything else",
                gb(headroom.max(0))
            ));
        } else if headroom < HEADROOM_YELLOW {
            state = state.worst(SafetyState::Yellow);
            reasons.push(format!("{} left for macOS and other apps", gb(headroom)));
        } else {
            state = state.worst(SafetyState::Green);
        }
    }

    if let Some(swap) = inputs.swap_used {
        if swap >= SWAP_RED {
            state = state.worst(SafetyState::Red);
            reasons.push(format!("{} of swap already in use", gb(swap as i64)));
        } else if swap >= SWAP_YELLOW {
            state = state.worst(SafetyState::Yellow);
            reasons.push(format!("{} of swap in use", gb(swap as i64)));
        }
    }

    Assessment {
        state,
        projected_used_bytes: projected,
        headroom_bytes: headroom,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Inputs {
        Inputs {
            installed: Some(32 * GIB),
            used: Some(10 * GIB),
            swap_used: Some(0),
            pressure: Pressure::Normal,
            running_model_bytes: None,
            predicted_total: None,
        }
    }

    #[test]
    fn nothing_readable_stays_unknown_rather_than_guessing() {
        let assessment = assess(Inputs::default());
        assert_eq!(assessment.state, SafetyState::Unknown);
        assert!(assessment.reasons.is_empty());
        assert_eq!(assessment.headroom_bytes, None);
    }

    #[test]
    fn a_quiet_machine_is_green() {
        assert_eq!(assess(base()).state, SafetyState::Green);
    }

    #[test]
    fn kernel_pressure_outranks_comfortable_headroom() {
        let inputs = Inputs {
            pressure: Pressure::Warning,
            ..base()
        };
        let assessment = assess(inputs);
        assert_eq!(assessment.state, SafetyState::Yellow);
        assert!(assessment.reasons[0].contains("elevated memory pressure"));

        let critical = assess(Inputs {
            pressure: Pressure::Critical,
            ..base()
        });
        assert_eq!(critical.state, SafetyState::Red);
    }

    #[test]
    fn a_launch_that_would_exhaust_memory_is_red_even_under_normal_pressure() {
        let assessment = assess(Inputs {
            predicted_total: Some(21 * GIB),
            ..base()
        });
        assert_eq!(assessment.state, SafetyState::Red);
        assert_eq!(assessment.projected_used_bytes, Some(31 * GIB));
        assert_eq!(assessment.headroom_bytes, Some(GIB as i64));
        assert!(assessment
            .reasons
            .iter()
            .any(|r| r.contains("left for macOS")));
    }

    #[test]
    fn a_tight_but_survivable_launch_is_yellow() {
        let assessment = assess(Inputs {
            predicted_total: Some(19 * GIB),
            ..base()
        });
        assert_eq!(assessment.state, SafetyState::Yellow);
        assert_eq!(assessment.headroom_bytes, Some(3 * GIB as i64));
    }

    #[test]
    fn the_model_being_replaced_is_not_counted_twice() {
        // 26 GB in use, of which 16 GB is the running model, replaced by a 20 GB one.
        let inputs = Inputs {
            used: Some(26 * GIB),
            running_model_bytes: Some(16 * GIB),
            predicted_total: Some(20 * GIB),
            ..base()
        };
        let assessment = assess(inputs);
        assert_eq!(assessment.projected_used_bytes, Some(30 * GIB));
        assert_eq!(assessment.state, SafetyState::Yellow);

        // Counting the running model twice would project 46 GB on a 32 GB machine and
        // report an ordinary model swap as impossible.
        let double_counted = assess(Inputs {
            running_model_bytes: None,
            ..inputs
        });
        assert_eq!(double_counted.projected_used_bytes, Some(46 * GIB));
        assert_eq!(double_counted.state, SafetyState::Red);
    }

    #[test]
    fn headroom_thresholds_are_exclusive() {
        let exactly_red_threshold = assess(Inputs {
            used: Some(30 * GIB),
            ..base()
        });
        assert_eq!(exactly_red_threshold.headroom_bytes, Some(HEADROOM_RED));
        assert_eq!(exactly_red_threshold.state, SafetyState::Yellow);

        let just_below = assess(Inputs {
            used: Some(30 * GIB + 1),
            ..base()
        });
        assert_eq!(just_below.state, SafetyState::Red);
    }

    #[test]
    fn swap_alone_can_raise_the_state() {
        let yellow = assess(Inputs {
            swap_used: Some(3 * GIB),
            ..base()
        });
        assert_eq!(yellow.state, SafetyState::Yellow);
        assert!(yellow.reasons.iter().any(|r| r.contains("swap")));

        let red = assess(Inputs {
            swap_used: Some(8 * GIB),
            ..base()
        });
        assert_eq!(red.state, SafetyState::Red);
    }

    #[test]
    fn one_unavailable_metric_does_not_suppress_the_others() {
        let no_installed = assess(Inputs {
            installed: None,
            pressure: Pressure::Critical,
            ..base()
        });
        assert_eq!(no_installed.state, SafetyState::Red);
        assert_eq!(no_installed.headroom_bytes, None);

        let no_pressure = assess(Inputs {
            pressure: Pressure::Unknown,
            predicted_total: Some(21 * GIB),
            ..base()
        });
        assert_eq!(no_pressure.state, SafetyState::Red);

        let no_swap = assess(Inputs {
            swap_used: None,
            ..base()
        });
        assert_eq!(no_swap.state, SafetyState::Green);
    }

    #[test]
    fn without_a_prediction_the_machine_is_assessed_as_it_stands() {
        let assessment = assess(base());
        assert_eq!(assessment.projected_used_bytes, Some(10 * GIB));
        assert_eq!(assessment.headroom_bytes, Some(22 * GIB as i64));
    }

    #[test]
    fn worst_signal_wins_across_all_three_sources() {
        let assessment = assess(Inputs {
            pressure: Pressure::Warning,
            swap_used: Some(3 * GIB),
            predicted_total: Some(21 * GIB),
            ..base()
        });
        assert_eq!(assessment.state, SafetyState::Red);
        assert_eq!(assessment.reasons.len(), 3);
    }
}
