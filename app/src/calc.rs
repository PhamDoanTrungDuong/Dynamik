//! Quick math for the Ask box: "12*(3+4)=" answers at once, on this PC.
//! Numbers with a decimal point or comma, + - * / ^ %, brackets and a leading minus.

/// The value of `expr` (an "=" at the end is allowed), or None when it isn't a sum.
pub fn eval(expr: &str) -> Option<f64> {
    let text: Vec<char> = expr.trim().trim_end_matches('=').chars().filter(|c| !c.is_whitespace()).collect();
    if text.is_empty() || !text.iter().any(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut p = Parser { s: &text, i: 0 };
    let v = p.sum()?;
    (p.i == text.len() && v.is_finite()).then_some(v)
}

/// A currency question, "100usd to vnd=" or "1,5 EUR in jpy": (amount, FROM, TO) in ISO codes.
pub fn currency(q: &str) -> Option<(f64, String, String)> {
    let s = q.trim().trim_end_matches('=').trim().to_lowercase();
    let (left, right) = s.split_once(" to ").or_else(|| s.split_once(" in "))?;
    let (left, to) = (left.trim(), right.trim());
    let letters = left.chars().rev().take_while(|c| c.is_ascii_alphabetic()).count();
    let (num, from) = left.split_at(left.len() - letters); // letters are ASCII, so this is a char boundary
    let code = |c: &str| c.len() == 3 && c.chars().all(|ch| ch.is_ascii_alphabetic());
    if !code(from) || !code(to) {
        return None;
    }
    Some((eval(num)?, from.to_uppercase(), to.to_uppercase()))
}

/// Money as people write it: thousands grouped, two decimals below 1000, none above.
pub fn money(v: f64) -> String {
    let rounded = if v.abs() >= 1000.0 { v.round() } else { (v * 100.0).round() / 100.0 };
    let s = show(rounded);
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let (sign, digits) = int.strip_prefix('-').map_or(("", int), |d| ("-", d));
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    if frac.is_empty() { format!("{sign}{grouped}") } else { format!("{sign}{grouped}.{frac}") }
}

/// The answer as people write it: no trailing zeros, at most 10 decimals.
pub fn show(v: f64) -> String {
    let s = format!("{:.10}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

struct Parser<'a> {
    s: &'a [char],
    i: usize,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(c);
        self.i += hit as usize;
        hit
    }

    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        loop {
            if self.eat('+') {
                v += self.product()?;
            } else if self.eat('-') {
                v -= self.product()?;
            } else {
                return Some(v);
            }
        }
    }

    fn product(&mut self) -> Option<f64> {
        let mut v = self.power()?;
        loop {
            if self.eat('*') || self.eat('×') || self.eat('x') {
                v *= self.power()?;
            } else if self.eat('/') || self.eat('÷') || self.eat(':') {
                let d = self.power()?;
                if d == 0.0 {
                    return None;
                }
                v /= d;
            } else {
                return Some(v);
            }
        }
    }

    /// `^` binds tighter than * and groups right: 2^3^2 = 2^9.
    fn power(&mut self) -> Option<f64> {
        let base = self.unary()?;
        if self.eat('^') {
            return Some(base.powf(self.power()?));
        }
        Some(base)
    }

    fn unary(&mut self) -> Option<f64> {
        if self.eat('-') {
            return Some(-self.unary()?);
        }
        if self.eat('+') {
            return self.unary();
        }
        let mut v = self.atom()?;
        // Shorthand people write for big amounts: 500k, 2tr / 2 triệu (millions), 1 tỷ (billions).
        let rest: String = self.s[self.i..].iter().collect::<String>().to_lowercase();
        for (word, times) in [("triệu", 1e6), ("tr", 1e6), ("tỷ", 1e9), ("tỉ", 1e9), ("k", 1e3)] {
            if rest.starts_with(word) {
                self.i += word.chars().count();
                v *= times;
                break;
            }
        }
        while self.eat('%') {
            v /= 100.0;
        }
        Some(v)
    }

    fn atom(&mut self) -> Option<f64> {
        if self.eat('(') {
            let v = self.sum()?;
            return self.eat(')').then_some(v);
        }
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.' || c == ',') {
            self.i += 1;
        }
        let raw: String = self.s[start..self.i].iter().collect();
        number(&raw)
    }
}

/// A written number, with "." or "," as the decimal mark or as thousands, as people write them:
/// both marks → the last one is the decimal (1.000,5 / 1,000.5); one mark used more than once →
/// thousands (1.000.000 / 2,500,000); one mark once, with exactly three digits after and not after
/// a lone 0 → thousands (1,000 / 25.000); otherwise the decimal (1,5 / 0.125 / 3.14).
/// shortcut: "3.141" reads as 3141 by that rule; a sum needing it can write 3.1410.
fn number(raw: &str) -> Option<f64> {
    let (dots, commas) = (raw.matches('.').count(), raw.matches(',').count());
    let digits = |s: &str| s.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    let text = if dots > 0 && commas > 0 {
        let at = raw.rfind(['.', ',']).unwrap();
        format!("{}.{}", digits(&raw[..at]), digits(&raw[at + 1..]))
    } else if dots + commas > 1 {
        // Thousands only if every group after the first has three digits.
        let groups: Vec<&str> = raw.split(['.', ',']).collect();
        if !groups[1..].iter().all(|g| g.len() == 3) {
            return None;
        }
        digits(raw)
    } else if dots + commas == 1 {
        let (int, frac) = raw.split_once(['.', ',']).unwrap();
        if frac.len() == 3 && !int.is_empty() && int != "0" { format!("{int}{frac}") } else { format!("{int}.{frac}") }
    } else {
        raw.to_string()
    };
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_follow_the_usual_order() {
        assert_eq!(eval("12*(3+4)="), Some(84.0));
        assert_eq!(eval("2+3*4"), Some(14.0));
        assert_eq!(eval("2^3^2"), Some(512.0));
        assert_eq!(eval("-5+2"), Some(-3.0));
        assert_eq!(eval("10 / 4"), Some(2.5));
        assert_eq!(eval("1,5 x 2 ="), Some(3.0));
        assert_eq!(eval("50% * 80"), Some(40.0));
    }

    #[test]
    fn numbers_written_with_thousands() {
        assert_eq!(eval("1,000*2="), Some(2000.0));
        assert_eq!(eval("1.000.000 / 4"), Some(250000.0));
        assert_eq!(eval("2,500,000 + 1"), Some(2500001.0));
        assert_eq!(eval("1.234,5 * 2"), Some(2469.0)); // Vietnamese / European
        assert_eq!(eval("1,234.5 * 2"), Some(2469.0)); // English
        assert_eq!(eval("1,5 + 1"), Some(2.5));
        assert_eq!(eval("0.125 * 8"), Some(1.0));
        assert_eq!(eval("3.14 * 2"), Some(6.28));
        assert_eq!(eval("1.00.0"), None); // groups that aren't thousands
        assert_eq!(currency("1.000.000 vnd to usd="), Some((1000000.0, "VND".into(), "USD".into())));
        // Shorthand for big amounts.
        assert_eq!(eval("500k + 1"), Some(500001.0));
        assert_eq!(eval("2tr * 2"), Some(4000000.0));
        assert_eq!(eval("2 triệu / 4"), Some(500000.0));
        assert_eq!(eval("1,5 tỷ"), Some(1500000000.0));
        assert_eq!(currency("500k vnd to usd="), Some((500000.0, "VND".into(), "USD".into())));
        assert_eq!(currency("2tr vnd in usd"), Some((2000000.0, "VND".into(), "USD".into())));
        assert_eq!(eval("ok"), None);
    }

    #[test]
    fn not_sums_give_nothing() {
        assert_eq!(eval("what is the weather"), None);
        assert_eq!(eval("1/0"), None);
        assert_eq!(eval("(1+2"), None);
        assert_eq!(eval("2+"), None);
        assert_eq!(eval("="), None);
    }

    #[test]
    fn currency_questions() {
        assert_eq!(currency("100usd to vnd="), Some((100.0, "USD".into(), "VND".into())));
        assert_eq!(currency("1,5 EUR in jpy"), Some((1.5, "EUR".into(), "JPY".into())));
        assert_eq!(currency("(2+3) gbp to usd ="), Some((5.0, "GBP".into(), "USD".into())));
        assert_eq!(currency("100 dollars to vnd"), None);
        assert_eq!(currency("12*3="), None);
        assert_eq!(currency("go to bed"), None);
        assert_eq!(money(2540050.4), "2,540,050");
        assert_eq!(money(92.456), "92.46");
        assert_eq!(money(-1234.0), "-1,234");
    }

    #[test]
    fn answers_read_cleanly() {
        assert_eq!(show(84.0), "84");
        assert_eq!(show(2.5), "2.5");
        assert_eq!(show(1.0 / 3.0), "0.3333333333");
        assert_eq!(show(-0.0), "0");
    }
}
