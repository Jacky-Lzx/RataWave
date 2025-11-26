use std::ops::{Add, AddAssign, Mul, Sub, SubAssign};
use std::{cmp::max, fmt::Display, str::FromStr};

use vcd::TimescaleUnit;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
/// Time stored in ps
pub struct Time(u64);

#[derive(Debug, PartialEq, Eq)]
pub struct ParseTimeError(String);

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut t: f64 = self.0 as f64;
        // let mut scale = TimescaleUnit::PS;
        use TimescaleUnit::*;
        let scales = [PS, NS, US, MS, S];
        let scale = scales
            .iter()
            .rfind(|x| t >= (PS.divisor() / x.divisor()) as f64)
            .unwrap_or(&PS);
        t /= (PS.divisor() / scale.divisor()) as f64;
        write!(f, "{}{}", t, scale)
    }
}

impl Add<u64> for Time {
    type Output = Time;

    fn add(self, rhs: u64) -> Self::Output {
        Time(self.0 + rhs)
    }
}

impl FromStr for Time {
    type Err = ParseTimeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseTimeError("Empty string".to_string()));
        }

        let split_index: usize = s
            .find(|x: char| !(x.is_ascii_digit() || x == '.'))
            .ok_or(ParseTimeError("Split error".to_string()))?;

        let (time, unit) = s.split_at(split_index);

        let time = time
            .parse::<f64>()
            .map_err(|_| ParseTimeError("Parse time error".to_string()))?;
        let unit = TimescaleUnit::from_str(unit.trim())
            .map_err(|_| ParseTimeError("Parse unit error".to_string()))?;

        if unit == TimescaleUnit::FS {
            return Err(ParseTimeError("Not support FS time scale".to_string()));
        }

        let time = time * (TimescaleUnit::PS.divisor() / unit.divisor()) as f64;
        if time.fract() != 0.0 {
            return Err(ParseTimeError("Time must be an integer in ps".to_string()));
        }
        let time = time.trunc() as u64;

        Ok(Time(time))
    }
}

impl Time {
    pub fn new(time: u64, unit: TimescaleUnit) -> Self {
        let time_in_ps = time * TimescaleUnit::PS.divisor() / unit.divisor();
        Time(time_in_ps)
    }

    pub fn time(&self) -> u64 {
        self.0
    }

    pub fn zero() -> Self {
        Time(0)
    }

    pub fn formulate(&self) -> u64 {
        let mut t = self.0;
        while t >= 1000 {
            if !t.is_multiple_of(1000) {
                panic!("self.time can not divides 1000!")
            }
            t /= 1000;
        }
        t
    }

    pub fn step_decrease(&mut self) {
        self.0 = match self.formulate() {
            1 | 10 | 100 => max(1, self.0 / 2),
            5 | 50 | 500 => self.0 / 5,
            _ => panic!("Invalid time step: {}", self.0),
        }
    }
    pub fn step_increase(&mut self) {
        self.0 = match self.formulate() {
            1 | 10 | 100 => self.0 * 5,
            5 | 50 | 500 => self.0 * 2,
            _ => panic!("Invalid time step: {}", self.0),
        }
    }

    /// Check if the given string is a valid time
    /// E.g. "100ns" or "100 ns" is a valid time
    ///
    /// ```
    /// use rata_wave::time::Time;
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

impl ParseTimeError {
    pub fn message(&self) -> &str {
        self.0.as_str()
    }
}
impl Display for ParseTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseTimeTrror: {}", self.0)
    }
}

impl Add for Time {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Time(self.0 + other.0)
    }
}

impl Mul for Time {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Time(self.0 * other.0)
    }
}

impl Mul<usize> for Time {
    type Output = Self;

    fn mul(self, other: usize) -> Self {
        Time(self.0 * other as u64)
    }
}

impl Sub for Time {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Time(self.0.saturating_sub(other.0))
    }
}

impl SubAssign for Time {
    fn sub_assign(&mut self, other: Self) {
        self.0 = self.0.saturating_sub(other.0);
    }
}

impl AddAssign for Time {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}
