use regex::Regex;
use tabwriter::TabWriter;

use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;

use utils::{drop_commas_and_parse, fmt_thousands_sep};

/// All extractable data from a single micro-benchmark.
#[derive(Debug)]
pub struct Benchmark {
    pub name: String,
    pub ns: usize,
    pub variance: usize,
    pub throughput: Option<usize>,
}

/// A comparison between an old and a new benchmark.
/// All differences are reported in terms of measuring improvements
/// (negative) or regressions (positive). That is, if an old benchmark
/// is slower than a new benchmark, then the difference is negative.
/// Conversely, if an old benchmark is faster than a new benchmark,
/// then the difference is positive.
#[derive(Debug)]
pub struct Comparison {
    pub fst: Benchmark,
    pub snd: Benchmark,
    pub diff_ns: i64,
    pub diff_ratio: f64,
}

const BENCHMARK_REGEX: &'static str = concat!(r#"test\s+(?P<name>\S+)\s+"#,
                                              r#"... bench:\s+(?P<ns>[0-9,]+)\s+ns/iter"#,
                                              r#"\s+\(\+/-\s+(?P<variance>[0-9,]+)\)"#,
                                              r#"(?:\s+=\s+(?P<throughput>[0-9,]+))?"#);

impl Benchmark {
    /// Parses a single benchmark line into a Benchmark.
    pub fn parse(line: String) -> Option<Benchmark> {
        lazy_static! {
            static ref RE: Regex = Regex::new(BENCHMARK_REGEX).unwrap();
        }
        RE.captures(line.as_str()).map(|c| {
            Benchmark {
                name: c.name("name").unwrap().into(),
                ns: c.name("ns").and_then(drop_commas_and_parse).unwrap(),
                variance: c.name("variance").and_then(drop_commas_and_parse).unwrap(),
                throughput: c.name("throughput").map(|t| drop_commas_and_parse(t).unwrap()),
            }
        })
    }

    /// Compares an old benchmark (self) with a new benchmark.
    pub fn compare(self, new: Self) -> Comparison {
        let diff_ns = new.ns as i64 - self.ns as i64;
        let diff_ratio = diff_ns as f64 / self.ns as f64;
        Comparison {
            fst: self,
            snd: new,
            diff_ns: diff_ns,
            diff_ratio: diff_ratio,
        }
    }
}

impl Comparison {
    pub fn write<W: Write>(&self, tw: &mut TabWriter<W>, variance: bool) -> io::Result<()> {
        macro_rules! w {
            ($($tt:tt)*) => { try!(write!(tw, $($tt)*)) }
        }
        w!("{}\t", self.fst.name);

        w!("{}", fmt_thousands_sep(self.fst.ns, ','));
        if variance {
            w!(" (+/- {})", self.fst.variance);
        }
        w!("\t");

        w!("{}", fmt_thousands_sep(self.snd.ns, ','));
        if variance {
            w!(" (+/- {})", self.snd.variance);
        }
        w!("\t");

        if self.diff_ns < 0 {
            w!("-");
        }
        w!("{}\t", fmt_thousands_sep(self.diff_ns.abs() as usize, ','));
        w!("{:.2}%\n", self.diff_ratio * 100f64);

        Ok(())
    }
}

pub fn parse_benchmarks(all_benchmarks: File) -> Box<Iterator<Item = Benchmark>> {
    let reader = BufReader::new(all_benchmarks);

    let lines = reader.lines().skip_while(|r| match *r {
        Ok(ref s) => s.is_empty(),
        _ => true,
    });

    Box::new(lines.filter_map(Result::ok)
        .filter_map(Benchmark::parse))
}
