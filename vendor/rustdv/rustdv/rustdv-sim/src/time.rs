//! Simulation time (design-doc §3.1: time as u64 steps + precision).

use rustdv_gpi as gpi;

/// A span of simulation time in simulator precision steps. Unit-safe
/// constructors (mapping row 13): `SimDuration::ns(2)`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimDuration {
    pub steps: u64,
}

impl SimDuration {
    pub fn steps(steps: u64) -> SimDuration {
        SimDuration { steps }
    }

    fn from_pow10(n: u64, exp: i32) -> SimDuration {
        let prec = gpi::time_precision();
        if exp >= prec {
            let factor = 10u64.pow((exp - prec) as u32);
            SimDuration { steps: n * factor }
        } else {
            let div = 10u64.pow((prec - exp) as u32);
            assert!(
                n.is_multiple_of(div),
                "duration {n}e{exp} is below simulator precision 1e{prec}"
            );
            SimDuration { steps: n / div }
        }
    }

    pub fn fs(n: u64) -> SimDuration {
        Self::from_pow10(n, -15)
    }
    pub fn ps(n: u64) -> SimDuration {
        Self::from_pow10(n, -12)
    }
    pub fn ns(n: u64) -> SimDuration {
        Self::from_pow10(n, -9)
    }
    pub fn us(n: u64) -> SimDuration {
        Self::from_pow10(n, -6)
    }
    pub fn ms(n: u64) -> SimDuration {
        Self::from_pow10(n, -3)
    }
    pub fn sec(n: u64) -> SimDuration {
        Self::from_pow10(n, 0)
    }

    /// Parse a cocotb-style unit name ("ns", "us", ...).
    pub fn from_unit(n: u64, unit: &str) -> SimDuration {
        match unit {
            "fs" => Self::fs(n),
            "ps" => Self::ps(n),
            "ns" => Self::ns(n),
            "us" => Self::us(n),
            "ms" => Self::ms(n),
            "s" | "sec" => Self::sec(n),
            "step" | "steps" => Self::steps(n),
            _ => panic!("unknown time unit '{unit}'"),
        }
    }
}

/// Current simulation time in precision steps.
pub fn sim_time_steps() -> u64 {
    gpi::sim_time_steps()
}

/// Current simulation time in nanoseconds (for log formatting).
pub fn sim_time_ns() -> f64 {
    let prec = gpi::time_precision();
    let steps = gpi::sim_time_steps() as f64;
    steps * 10f64.powi(prec + 9)
}
