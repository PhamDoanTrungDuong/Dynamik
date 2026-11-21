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
        let num: String = self.s[start..self.i].iter().map(|&c| if c == ',' { '.' } else { c }).collect();
        num.parse().ok()
    }
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
    fn not_sums_give_nothing() {
        assert_eq!(eval("what is the weather"), None);
        assert_eq!(eval("1/0"), None);
        assert_eq!(eval("(1+2"), None);
        assert_eq!(eval("2+"), None);
        assert_eq!(eval("="), None);
    }

    #[test]
    fn answers_read_cleanly() {
        assert_eq!(show(84.0), "84");
        assert_eq!(show(2.5), "2.5");
        assert_eq!(show(1.0 / 3.0), "0.3333333333");
        assert_eq!(show(-0.0), "0");
    }
}
