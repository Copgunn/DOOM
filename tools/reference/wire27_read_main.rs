//! Rust side of tools/reference/wire27_read.py: the repository's intercept_vector.rs, unmodified (by path),
//! and sight.rs's checked_intercept (its lines, extracted by the tool), driven from stdin. The tool copies this
//! file into its work directory and replaces the two path tokens; it is not a workspace member.
#![allow(dead_code, unused_imports)]
use std::io::{BufRead, Write};

pub use doom_types::Fixed;
pub struct StaticWorld {
    pub(crate) wrapping_intercept: bool,
}
mod sight {
    use crate::Fixed;
    include!("@EXTRACT@");
}
#[path = "@INTERCEPT@"]
mod intercept_vector;
use intercept_vector::{InterceptCaller, policy_intercept, sight_intercept, wire27_intercept};

fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::with_capacity(1 << 16, stdout.lock());
    let on = StaticWorld {
        wrapping_intercept: true,
    };
    let off = StaticWorld {
        wrapping_intercept: false,
    };
    let callers = [
        InterceptCaller::AimLine,
        InterceptCaller::AimThing,
        InterceptCaller::ShootLine,
        InterceptCaller::ShootThing,
        InterceptCaller::Slide,
        InterceptCaller::Use,
    ];
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let v: Vec<i32> = line
            .split_whitespace()
            .map(|t| t.parse().unwrap())
            .collect();
        let f = |i: usize| [Fixed::from_bits(v[i]), Fixed::from_bits(v[i + 1])];
        let (s2, d2, s1, d1) = (f(0), f(2), f(4), f(6));
        let checked = sight::checked_intercept(s2, d2, s1, d1);
        match checked {
            Some(x) => write!(out, "S{}", x.bits()).unwrap(),
            None => write!(out, "N").unwrap(),
        }
        for c in callers {
            // The flag off must be checked_intercept exactly, for every caller.
            assert_eq!(policy_intercept(&off, c, s2, d2, s1, d1), checked);
            let r = wire27_intercept(c, s2, d2, s1, d1);
            assert_eq!(policy_intercept(&on, c, s2, d2, s1, d1), r.ok());
            match r {
                Ok(x) => write!(out, " O{}", x.bits()).unwrap(),
                Err(e) => write!(out, " E{}", format!("{e:?}").replace(' ', "")).unwrap(),
            }
        }
        assert_eq!(sight_intercept(false, s2, d2, s1, d1), checked.map(Ok));
        match sight_intercept(true, s2, d2, s1, d1) {
            None => writeln!(out, " N").unwrap(),
            Some(Ok(x)) => writeln!(out, " O{}", x.bits()).unwrap(),
            Some(Err(e)) => writeln!(out, " E{e:?}").unwrap(),
        }
    }
}
