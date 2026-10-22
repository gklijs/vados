//! A page's publication date, as page.json's `date` field gives it: an ISO
//! 8601 calendar date, `YYYY-MM-DD`, optionally followed by a time of day,
//! `THH:MM` or `THH:MM:SS`, so posts published on the same day can still be
//! told apart. No time zone: the date is the site's own local one. See
//! `vados.allium`'s `PageDate`.

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageDate {
    year: u16,
    month: u8,
    day: u8,
    /// Hour, minute and second; absent when only a date was given.
    time: Option<(u8, u8, u8)>,
    /// Whether the time was written with seconds, so it is written back the
    /// way it was read.
    has_seconds: bool,
}

impl PageDate {
    pub fn year(&self) -> u16 {
        self.year
    }
    /// 1 for January through 12 for December.
    pub fn month(&self) -> u8 {
        self.month
    }
    pub fn day(&self) -> u8 {
        self.day
    }

    /// Today's date in UTC. Used as the default date for a new page in a
    /// section ordered by date.
    pub fn today() -> PageDate {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
        PageDate {
            year,
            month,
            day,
            time: None,
            has_seconds: false,
        }
    }

    /// The RFC 3339 timestamp feeds need, which must carry a time zone. A
    /// page date has none, so it's read as UTC, and a date without a time as
    /// the start of that day.
    pub fn to_rfc3339(&self) -> String {
        let (h, m, s) = self.time.unwrap_or_default();
        format!(
            "{:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z",
            self.year, self.month, self.day
        )
    }

    fn sort_key(&self) -> (u16, u8, u8, (u8, u8, u8)) {
        (
            self.year,
            self.month,
            self.day,
            self.time.unwrap_or_default(),
        )
    }
}

/// Chronological. A date without a time counts as the start of that day.
impl Ord for PageDate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for PageDate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The ISO 8601 form, exactly as page.json would hold it -- also what a
/// `<time datetime>` attribute takes.
impl fmt::Display for PageDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)?;
        match self.time {
            Some((h, m, s)) if self.has_seconds => write!(f, "T{h:02}:{m:02}:{s:02}"),
            Some((h, m, _)) => write!(f, "T{h:02}:{m:02}"),
            None => Ok(()),
        }
    }
}

impl FromStr for PageDate {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || format!("`{s}` is not a date of the form YYYY-MM-DD or YYYY-MM-DDTHH:MM");
        let (date, time) = match s.trim().split_once('T') {
            Some((d, t)) => (d, Some(t)),
            None => (s.trim(), None),
        };

        let parts: Vec<&str> = date.split('-').collect();
        let [year, month, day] = parts[..] else {
            return Err(invalid());
        };
        if year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return Err(invalid());
        }
        let year: u16 = number(year).ok_or_else(invalid)?;
        let month: u8 = number(month).ok_or_else(invalid)?;
        let day: u8 = number(day).ok_or_else(invalid)?;
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return Err(format!("`{s}` is not a day that exists"));
        }

        let (time, has_seconds) = match time {
            None => (None, false),
            Some(t) => {
                let parts: Vec<&str> = t.split(':').collect();
                if !(2..=3).contains(&parts.len()) || parts.iter().any(|p| p.len() != 2) {
                    return Err(invalid());
                }
                let hour: u8 = number(parts[0]).ok_or_else(invalid)?;
                let minute: u8 = number(parts[1]).ok_or_else(invalid)?;
                let second: u8 = match parts.get(2) {
                    Some(p) => number(p).ok_or_else(invalid)?,
                    None => 0,
                };
                if hour > 23 || minute > 59 || second > 59 {
                    return Err(format!("`{s}` is not a time that exists"));
                }
                (Some((hour, minute, second)), parts.len() == 3)
            }
        };

        Ok(PageDate {
            year,
            month,
            day,
            time,
            has_seconds,
        })
    }
}

/// Digits only: `str::parse` alone would also accept a leading `+`.
fn number<T: FromStr>(s: &str) -> Option<T> {
    if s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The proleptic Gregorian date `days` after 1970-01-01, after Howard
/// Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (u16, u8, u8) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year as u16, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> PageDate {
        s.parse().unwrap()
    }

    #[test]
    fn parses_a_date_and_writes_it_back_unchanged() {
        for s in ["2026-10-22", "2026-10-22T09:30", "2026-10-22T09:30:15"] {
            assert_eq!(date(s).to_string(), s);
        }
    }

    #[test]
    fn rejects_malformed_and_impossible_dates() {
        for s in [
            "",
            "22-10-2026",
            "2026-1-22",
            "2026/10/22",
            "2026-10-22 09:30",
            "2026-10-22T9:30",
            "2026-10-22T09",
            "2026-+1-22",
            "2026-13-01",
            "2026-02-29",
            "2026-04-31",
            "2026-10-00",
            "2026-10-22T24:00",
            "2026-10-22T12:60",
        ] {
            assert!(s.parse::<PageDate>().is_err(), "{s} should be rejected");
        }
    }

    #[test]
    fn leap_days_follow_the_gregorian_rules() {
        assert!("2024-02-29".parse::<PageDate>().is_ok());
        assert!("2000-02-29".parse::<PageDate>().is_ok());
        assert!("1900-02-29".parse::<PageDate>().is_err());
    }

    #[test]
    fn orders_chronologically_with_time_breaking_same_day_ties() {
        assert!(date("2026-10-22") > date("2026-10-21T23:59"));
        assert!(date("2026-10-22T10:00") > date("2026-10-22T09:59:59"));
        assert!(date("2026-10-22") < date("2026-10-22T00:01"));
        assert!(date("2025-12-31") < date("2026-01-01"));
    }

    #[test]
    fn rfc3339_reads_the_date_as_utc_with_a_full_time() {
        assert_eq!(date("2026-10-22").to_rfc3339(), "2026-10-22T00:00:00Z");
        assert_eq!(
            date("2026-10-22T09:30").to_rfc3339(),
            "2026-10-22T09:30:00Z"
        );
        assert_eq!(
            date("2026-10-22T09:30:15").to_rfc3339(),
            "2026-10-22T09:30:15Z"
        );
    }

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(20_748), (2026, 10, 22));
    }
}
