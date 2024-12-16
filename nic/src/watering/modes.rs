use super::ds::DailyPlan;
use num_derive::FromPrimitive;
use std::fmt::{Debug, Display};

#[derive(Clone, Copy, Debug, PartialEq, FromPrimitive)]
#[repr(usize)]
pub enum Mode {
    Auto = 0,
    Manual = 1,
    Wizard = 2,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mode = match self {
            Mode::Auto => "auto",
            Mode::Manual => "manual",
            Mode::Wizard => "wizard",
        };
        f.write_str(mode)
    }
}

impl std::str::FromStr for Mode {
    type Err = &'static str;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "auto" => Ok(Mode::Auto),
            "manual" => Ok(Mode::Manual),
            "wizard" => Ok(Mode::Wizard),
            _ => Err("Invalid mode"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModeAuto {
    pub daily_plan: Vec<DailyPlan>, // Store the schedule here
}

#[derive(Clone, Debug)]
pub struct ModeWizard {
    pub daily_plan: Vec<DailyPlan>,
}

#[derive(Clone, Debug)]
pub struct ModeManual;
