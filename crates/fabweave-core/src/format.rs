//! Écriture des nombres dans le G-code : décimales fixes, zéros inutiles
//! retirés, jamais de `-0`.

/// `v` avec au plus `decimals` décimales (`12.5`, `0.04`, `-3`).
pub fn num(v: f64, decimals: usize) -> String {
    let mut s = format!("{v:.decimals$}");
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.').len();
        s.truncate(trimmed);
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// Durée lisible façon PrusaSlicer : `1h 2m 3s`, `4m 5s`, `6s`.
pub fn duration(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    let (d, h, m, s) = (
        total / 86_400,
        total / 3600 % 24,
        total / 60 % 60,
        total % 60,
    );
    let mut out = String::new();
    if d > 0 {
        out.push_str(&format!("{d}d "));
    }
    if d > 0 || h > 0 {
        out.push_str(&format!("{h}h "));
    }
    if d > 0 || h > 0 || m > 0 {
        out.push_str(&format!("{m}m "));
    }
    out.push_str(&format!("{s}s"));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_trimmed() {
        assert_eq!(num(12.5, 3), "12.5");
        assert_eq!(num(0.04, 5), "0.04");
        assert_eq!(num(-0.0001, 3), "0");
        assert_eq!(num(100.0, 3), "100");
        assert_eq!(num(1.23456, 3), "1.235");
    }

    #[test]
    fn durations_read_like_prusaslicer() {
        assert_eq!(duration(3723.4), "1h 2m 3s");
        assert_eq!(duration(65.0), "1m 5s");
        assert_eq!(duration(6.0), "6s");
        assert_eq!(duration(3600.0), "1h 0m 0s");
    }
}
