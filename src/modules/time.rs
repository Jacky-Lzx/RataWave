use std::{cmp::max, fmt::Display, ops::Add, str::FromStr};

use cli_log::debug;
use vcd::TimescaleUnit;

#[derive(Clone)]
pub struct Time {
    // Stored in ps
    time: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseTimeError {
    message: String,
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut t: f64 = self.time as f64;
        // let mut scale = TimescaleUnit::PS;
        use TimescaleUnit::*;
        let scales = [PS, NS, US, MS, S];
        let scale = scales
            .iter()
            .rfind(|x| t >= (PS.divisor() / x.divisor()) as f64)
            .unwrap_or(&PS);
        t = t / (PS.divisor() / scale.divisor()) as f64;
        write!(f, "{}{}", t, scale)
    }
}

impl Add<u64> for Time {
    type Output = Time;

    fn add(self, rhs: u64) -> Self::Output {
        Time {
            time: self.time + rhs,
        }
    }
}

impl FromStr for Time {
    type Err = ParseTimeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.len() == 0 {
            return Err(ParseTimeError {
                message: "Empty string".to_string(),
            });
        }

        let split_index: usize =
            s.find(|x: char| !(x.is_ascii_digit() || x == '.'))
                .ok_or(ParseTimeError {
                    message: "Split error".to_string(),
                })?;

        let (time, unit) = s.split_at(split_index);

        let time = time.parse::<f64>().map_err(|_| ParseTimeError {
            message: "Parse time error".to_string(),
        })?;
        let unit = TimescaleUnit::from_str(unit.trim()).map_err(|_| ParseTimeError {
            message: "Parse unit error".to_string(),
        })?;

        if unit == TimescaleUnit::FS {
            return Err(ParseTimeError {
                message: "Not support FS time scale".to_string(),
            });
        }

        let time = time * (TimescaleUnit::PS.divisor() / unit.divisor()) as f64;
        debug!("Time: {}", time);
        if time.fract() != 0.0 {
            return Err(ParseTimeError {
                message: "Time must be an integer in ps".to_string(),
            });
        }
        let time = time.trunc() as u64;

        Ok(Time { time })
    }
}

impl Time {
    pub fn new(time: u64, unit: TimescaleUnit) -> Self {
        let time_in_ps = time * TimescaleUnit::PS.divisor() / unit.divisor();
        Time { time: time_in_ps }
    }

    pub fn increase(&mut self, time_inc: u64) {
        self.time += time_inc;
    }

    pub fn decrease(&mut self, time_dec: u64) {
        self.time = if self.time < time_dec {
            0
        } else {
            self.time - time_dec
        }
    }

    pub fn time(&self) -> u64 {
        self.time
    }

    pub fn formulate(&self) -> u64 {
        let mut t = self.time;
        while t >= 1000 {
            if t % 1000 != 0 {
                panic!("self.time can not divides 1000!")
            }
            t /= 1000;
        }
        t
    }

    pub fn step_decrease(&mut self) {
        self.time = match self.formulate() {
            1 | 10 | 100 => max(1, self.time / 2),
            5 | 50 | 500 => self.time / 5,
            _ => panic!("Invalid time step: {}", self.time),
        }
    }
    pub fn step_increase(&mut self) {
        self.time = match self.formulate() {
            1 | 10 | 100 => self.time * 5,
            5 | 50 | 500 => self.time * 2,
            _ => panic!("Invalid time step: {}", self.time),
        }
    }

    /// Check if the given string is a valid time
    /// E.g. "100ns" or "100 ns" is a valid time
    ///
    /// ```
    /// use vcd_rs::time::Time;
    ///
    /// assert!(Time::is_valid("100ns").is_ok());
    /// assert!(Time::is_valid("100 ns").is_ok());
    /// assert!(Time::is_valid("0.5us").is_ok());
    /// assert!(Time::is_valid("100.001ns").is_ok());
    /// // Since 1ps is the smallest time, if the time representation is not an integer in ps it
    /// // will generate an error
    /// assert!(Time::is_valid("1ps").is_ok());
    /// assert!(Time::is_valid("0.1ps").is_err());
    /// assert!(Time::is_valid("100.0001ns").is_err());
    /// ```
    pub fn is_valid(s: &str) -> Result<(), ParseTimeError> {
        match Time::from_str(s) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl Display for ParseTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse time error: {}", self.message)
    }
}
