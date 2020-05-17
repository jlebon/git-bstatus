/* Copyright (C) 2018 Jonathan Lebon <jonathan@jlebon.com>
 * SPDX-License-Identifier: MIT
 * */

use std::time;

const SECONDS_PER_MINUTE: u64 = 60;
const MINUTES_PER_HOUR: u64 = 60;
const HOURS_PER_DAY: u64 = 24;
const DAYS_PER_WEEK: u64 = 7;
const DAYS_PER_MONTH: u64 = 30; // meh... good enough for our purposes
const MONTHS_PER_YEAR: u64 = 12;

pub fn epoch_to_relative_str(timestamp: u64) -> String {
    let timestamp = timestamp as u64;
    let now = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    if timestamp >= now {
        return "now".into();
    }

    let secs = now - timestamp;
    if secs < SECONDS_PER_MINUTE {
        return plural("sec", secs);
    }

    let mins = secs / SECONDS_PER_MINUTE;
    if mins < MINUTES_PER_HOUR {
        return plural("min", mins);
    }

    let hours = mins / MINUTES_PER_HOUR;
    if hours < HOURS_PER_DAY {
        return plural("hour", hours);
    }

    let days = hours / HOURS_PER_DAY;
    if days < DAYS_PER_WEEK {
        return plural("day", days);
    }

    if days < DAYS_PER_MONTH {
        return plural("week", days / DAYS_PER_WEEK);
    }

    let months = days / DAYS_PER_MONTH;
    if months < MONTHS_PER_YEAR {
        return plural("month", months);
    }

    let years = months / MONTHS_PER_YEAR;
    plural("year", years)
}

fn plural(s: &str, n: u64) -> String {
    format!("{} {}{}", n, s, if n == 1 { "" } else { "s" })
}

pub fn count_digits(mut n: usize) -> usize {
    match n {
        0..=9 => 1,
        10..=99 => 2,
        _ => {
            let mut i = 0;
            while n > 0 {
                n /= 10;
                i += 1;
            }
            i
        }
    }
}

#[test]
fn test_count_digits() {
    assert_eq!(1, count_digits(0));
    assert_eq!(1, count_digits(1));
    assert_eq!(1, count_digits(9));
    assert_eq!(2, count_digits(10));
    assert_eq!(2, count_digits(11));
    assert_eq!(2, count_digits(99));
    assert_eq!(3, count_digits(100));
    assert_eq!(3, count_digits(101));
    assert_eq!(3, count_digits(999));
    assert_eq!(4, count_digits(1000));
    assert_eq!(4, count_digits(1001));
}
