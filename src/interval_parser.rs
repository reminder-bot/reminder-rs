/*
With modifications, 2022 Jude Southworth

Original copyright notice:

Copyright 2021 Paul Colomiets

Permission is hereby granted, free of charge, to any person obtaining a copy of this software
and associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or
substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
*/

use std::{error::Error as StdError, fmt, str::Chars};

/// Error parsing human-friendly duration
#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    /// Invalid character during parsing
    ///
    /// More specifically anything that is not alphanumeric is prohibited
    ///
    /// The field is an byte offset of the character in the string.
    InvalidCharacter(usize),
    /// Non-numeric value where number is expected
    ///
    /// This usually means that either time unit is broken into words,
    /// e.g. `m sec` instead of `msec`, or just number is omitted,
    /// for example `2 hours min` instead of `2 hours 1 min`
    ///
    /// The field is an byte offset of the errorneous character
    /// in the string.
    NumberExpected(usize),
    /// Unit in the number is not one of allowed units
    ///
    /// See documentation of `parse_duration` for the list of supported
    /// time units.
    ///
    /// The two fields are start and end (exclusive) of the slice from
    /// the original string, containing errorneous value
    UnknownUnit {
        /// Start of the invalid unit inside the original string
        start: usize,
        /// End of the invalid unit inside the original string
        end: usize,
        /// The unit verbatim
        unit: String,
        /// A number associated with the unit
        value: u64,
    },
    /// The numeric value is too large
    ///
    /// Usually this means value is too large to be useful. If user writes
    /// data in subsecond units, then the maximum is about 3k years. When
    /// using seconds, or larger units, the limit is even larger.
    NumberOverflow,
    /// The value was an empty string (or consists only whitespace)
    Empty,
}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidCharacter(offset) => write!(f, "invalid character at {}", offset),
            Error::NumberExpected(offset) => write!(f, "expected number at {}", offset),
            Error::UnknownUnit { unit, value, .. } if unit.is_empty() => {
                write!(f, "time unit needed, for example {0}sec or {0}ms", value,)
            }
            Error::UnknownUnit { unit, .. } => {
                write!(
                    f,
                    "unknown time unit {:?}, \
                    supported units: ns, us, ms, sec, min, hours, days, \
                    weeks, months, years (and few variations)",
                    unit
                )
            }
            Error::NumberOverflow => write!(f, "number is too large"),
            Error::Empty => write!(f, "value was empty"),
        }
    }
}

trait OverflowOp: Sized {
    fn mul(self, other: Self) -> Result<Self, Error>;
    fn add(self, other: Self) -> Result<Self, Error>;
}

impl OverflowOp for u64 {
    fn mul(self, other: Self) -> Result<Self, Error> {
        self.checked_mul(other).ok_or(Error::NumberOverflow)
    }
    fn add(self, other: Self) -> Result<Self, Error> {
        self.checked_add(other).ok_or(Error::NumberOverflow)
    }
}

#[derive(Copy, Clone)]
pub struct Interval {
    pub month: u64,
    pub day: u64,
    pub sec: u64,
}

struct Parser<'a> {
    iter: Chars<'a>,
    src: &'a str,
    current: (u64, u64, u64, u64),
}

impl<'a> Parser<'a> {
    fn off(&self) -> usize {
        self.src.len() - self.iter.as_str().len()
    }

    fn parse_first_char(&mut self) -> Result<Option<u64>, Error> {
        let off = self.off();
        for c in self.iter.by_ref() {
            match c {
                '0'..='9' => {
                    return Ok(Some(c as u64 - '0' as u64));
                }
                c if c.is_whitespace() => continue,
                _ => {
                    return Err(Error::NumberExpected(off));
                }
            }
        }
        Ok(None)
    }
    fn parse_unit(&mut self, n: u64, start: usize, end: usize) -> Result<(), Error> {
        let (mut month, mut day, mut sec, nsec) = match &self.src[start..end] {
            "nanos" | "nsec" | "ns" => (0, 0u64, 0u64, n),
            "usec" | "us" => (0, 0, 0u64, n.mul(1000)?),
            "millis" | "msec" | "ms" => (0, 0, 0u64, n.mul(1_000_000)?),
            "seconds" | "second" | "secs" | "sec" | "s" => (0, 0, n, 0),
            "minutes" | "minute" | "min" | "mins" | "m" => (0, 0, n.mul(60)?, 0),
            "hours" | "hour" | "hr" | "hrs" | "h" => (0, 0, n.mul(3600)?, 0),
            "days" | "day" | "d" => (0, n, 0, 0),
            "weeks" | "week" | "w" => (0, n.mul(7)?, 0, 0),
            "months" | "month" | "M" => (n, 0, 0, 0),
            "years" | "year" | "y" => (n.mul(12)?, 0, 0, 0),
            _ => {
                return Err(Error::UnknownUnit {
                    start,
                    end,
                    unit: self.src[start..end].to_string(),
                    value: n,
                });
            }
        };
        let mut nsec = self.current.3 + nsec;
        if nsec > 1_000_000_000 {
            sec += nsec / 1_000_000_000;
            nsec %= 1_000_000_000;
        }
        sec += self.current.2;
        day += self.current.1;
        month += self.current.0;

        self.current = (month, day, sec, nsec);

        Ok(())
    }

    fn parse(mut self) -> Result<Interval, Error> {
        let mut n = self.parse_first_char()?.ok_or(Error::Empty)?;
        'outer: loop {
            let mut off = self.off();
            while let Some(c) = self.iter.next() {
                match c {
                    '0'..='9' => {
                        n = n
                            .checked_mul(10)
                            .and_then(|x| x.checked_add(c as u64 - '0' as u64))
                            .ok_or(Error::NumberOverflow)?;
                    }
                    c if c.is_whitespace() => {}
                    'a'..='z' | 'A'..='Z' => {
                        break;
                    }
                    _ => {
                        return Err(Error::InvalidCharacter(off));
                    }
                }
                off = self.off();
            }
            let start = off;
            let mut off = self.off();
            while let Some(c) = self.iter.next() {
                match c {
                    '0'..='9' => {
                        self.parse_unit(n, start, off)?;
                        n = c as u64 - '0' as u64;
                        continue 'outer;
                    }
                    c if c.is_whitespace() => break,
                    'a'..='z' | 'A'..='Z' => {}
                    _ => {
                        return Err(Error::InvalidCharacter(off));
                    }
                }
                off = self.off();
            }
            self.parse_unit(n, start, off)?;
            n = match self.parse_first_char()? {
                Some(n) => n,
                None => {
                    return Ok(Interval {
                        month: self.current.0,
                        day: self.current.1,
                        sec: self.current.2,
                    })
                }
            };
        }
    }
}

/// Parse duration object `1hour 12min 5s`
///
/// The duration object is a concatenation of time spans. Where each time
/// span is an integer number and a suffix. Supported suffixes:
///
/// * `nsec`, `ns` -- nanoseconds
/// * `usec`, `us` -- microseconds
/// * `msec`, `ms` -- milliseconds
/// * `seconds`, `second`, `sec`, `s`
/// * `minutes`, `minute`, `min`, `m`
/// * `hours`, `hour`, `hr`, `h`
/// * `days`, `day`, `d`
/// * `weeks`, `week`, `w`
/// * `months`, `month`, `M` -- defined as 30.44 days
/// * `years`, `year`, `y` -- defined as 365.25 days
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use humantime::parse_duration;
///
/// assert_eq!(parse_duration("2h 37min"), Ok(Duration::new(9420, 0)));
/// assert_eq!(parse_duration("32ms"), Ok(Duration::new(0, 32_000_000)));
/// ```
pub fn parse_duration(s: &str) -> Result<Interval, Error> {
    Parser { iter: s.chars(), src: s, current: (0, 0, 0, 0) }.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_seconds() {
        let interval = parse_duration("10 seconds").unwrap();

        assert_eq!(interval.sec, 10);
        assert_eq!(interval.day, 0);
        assert_eq!(interval.month, 0);
    }

    #[test]
    fn parse_minutes() {
        let interval = parse_duration("10 minutes").unwrap();

        assert_eq!(interval.sec, 600);
        assert_eq!(interval.day, 0);
        assert_eq!(interval.month, 0);
    }

    #[test]
    fn parse_hours() {
        let interval = parse_duration("10 hours").unwrap();

        assert_eq!(interval.sec, 36_000);
        assert_eq!(interval.day, 0);
        assert_eq!(interval.month, 0);
    }

    #[test]
    fn parse_days() {
        let interval = parse_duration("10 days").unwrap();

        assert_eq!(interval.sec, 0);
        assert_eq!(interval.day, 10);
        assert_eq!(interval.month, 0);
    }

    #[test]
    fn parse_weeks() {
        let interval = parse_duration("10 weeks").unwrap();

        assert_eq!(interval.sec, 0);
        assert_eq!(interval.day, 70);
        assert_eq!(interval.month, 0);
    }

    #[test]
    fn parse_months() {
        let interval = parse_duration("10 months").unwrap();

        assert_eq!(interval.sec, 0);
        assert_eq!(interval.day, 0);
        assert_eq!(interval.month, 10);
    }

    #[test]
    fn parse_years() {
        let interval = parse_duration("10 years").unwrap();

        assert_eq!(interval.sec, 0);
        assert_eq!(interval.day, 0);
        assert_eq!(interval.month, 120);
    }
}
