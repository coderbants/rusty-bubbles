//! Go-compatible `time.Duration::String()` formatting.
//!
//! Rust-side porting helper (not present in upstream bubbles): bubbletea's
//! stopwatch/timer components render durations via Go's `time.Duration`
//! formatting; this module replicates it exactly so example output matches
//! byte-for-byte.

use std::time::Duration;

/// Renders the given duration using Go's `time.Duration.String()` algorithm.
pub fn duration_string(d: Duration) -> String {
    let mut u = d.as_nanos() as i128;
    let neg = u < 0;
    if neg {
        u = -u;
    }

    let mut s = String::new();

    if u < 1_000_000_000 {
        // Special case: if duration is smaller than a second, use smaller
        // units, like 1.2ms.
        if u == 0 {
            return "0s".to_string();
        }
        let prec: usize;
        let unit: &str;
        if u < 1_000 {
            // print nanoseconds
            prec = 0;
            unit = "n";
        } else if u < 1_000_000 {
            // print microseconds (µ micro sign U+00B5)
            prec = 3;
            unit = "µ";
        } else {
            // print milliseconds
            prec = 6;
            unit = "m";
        }
        let mut digits: Vec<char> = Vec::new();
        fmt_frac(&mut digits, &mut u, prec);
        fmt_int(&mut digits, u);
        for c in digits.iter().rev() {
            s.push(*c);
        }
        s.push_str(unit);
        s.push('s');
    } else {
        // Go writes into the buffer from the end (w--), so the write order
        // is: 's', fraction, integer-seconds, 'm', integer-minutes, 'h',
        // integer-hours. The final string is the reverse of the write order.
        let mut digits: Vec<char> = Vec::new();
        digits.push('s');
        fmt_frac(&mut digits, &mut u, 9);

        // u is now integer seconds
        fmt_int(&mut digits, u % 60);
        u /= 60;

        // u is now integer minutes
        if u > 0 {
            digits.push('m');
            fmt_int(&mut digits, u % 60);
            u /= 60;

            // u is now integer hours
            // Stop at hours because days can be different lengths.
            if u > 0 {
                digits.push('h');
                fmt_int(&mut digits, u);
            }
        }
        for c in digits.iter().rev() {
            s.push(*c);
        }
    }

    if neg {
        s.insert(0, '-');
    }
    s
}

/// Formats the fraction of v/10**prec (e.g., ".12345"), omitting trailing
/// zeros. Digits are appended least-significant first; the caller reverses.
fn fmt_frac(digits: &mut Vec<char>, v: &mut i128, prec: usize) {
    // Omit trailing zeros up to and including decimal point.
    let mut print = false;
    for _ in 0..prec {
        let digit = *v % 10;
        print = print || digit != 0;
        if print {
            digits.push((digit as u8 + b'0') as char);
        }
        *v /= 10;
    }
    if print {
        digits.push('.');
    }
}

/// Formats v's decimal digits into the tail, least-significant first; the
/// caller reverses.
fn fmt_int(digits: &mut Vec<char>, v: i128) {
    if v == 0 {
        digits.push('0');
    } else {
        let mut v = v;
        while v > 0 {
            digits.push(((v % 10) as u8 + b'0') as char);
            v /= 10;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration_string_formats() {
        assert_eq!(duration_string(Duration::ZERO), "0s");
        assert_eq!(duration_string(Duration::from_nanos(500)), "500ns");
        assert_eq!(duration_string(Duration::from_micros(12)), "12µs");
        assert_eq!(duration_string(Duration::from_micros(1200)), "1.2ms");
        assert_eq!(duration_string(Duration::from_millis(500)), "500ms");
        assert_eq!(duration_string(Duration::from_secs(5)), "5s");
        assert_eq!(
            duration_string(Duration::from_secs(65) + Duration::from_millis(500)),
            "1m5.5s"
        );
        assert_eq!(
            duration_string(Duration::from_secs(3665) + Duration::from_micros(250000)),
            "1h1m5.25s"
        );
    }
}
