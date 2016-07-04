extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate tabwriter;

use docopt::Docopt;
use regex::Regex;
use tabwriter::TabWriter;

use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::fs::File;
use std::cmp::Ordering;
use std::cmp::Ordering::*;

const USAGE: &'static str = r#"
Compare Rust micro-benchmarks by saving the output of the benchmark to file
and providing it into this command.
The first version takes two file and compares the common bench-tests.
The second version takes two names of implementations and one or more files,
and compares the common bench-tests of the two implementations based on the
name scheme "implementation::test".

Usage: cargo-benchcmp [options] <file> <file>
       cargo-benchcmp [options] <name> <name> [--] <file>...
       cargo-benchcmp -h | --help


Options:
  -h, --help            show this help message and exit
  --threshold <n>       only show comparisons with a percentage change greater
                        than this threshold
  --variance            show variance
  --show <option>       show regressions, improvements or both [default: both]
  --strip-fst <regex>   a regex to strip from first benchmarks' names
  --strip-snd <regex>   a regex to strip from second benchmarks' names
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_threshold: Option<u8>,
    flag_variance: bool,
    flag_show: ShowOption,
    flag_strip_fst: Option<String>,
    flag_strip_snd: Option<String>,
    arg_name: Option<[String; 2]>,
    arg_file: Vec<String>,
}

#[derive(Debug, RustcDecodable, PartialEq, Eq)]
enum ShowOption {
    Regressions,
    Improvements,
    Both,
}

/// All extractable data from a single micro-benchmark.
#[derive(Debug)]
struct Benchmark {
    name: String,
    ns: usize,
    variance: usize,
    throughput: Option<usize>,
}

const BENCHMARK_REGEX: &'static str = concat!(r#"test\s+(?P<name>\S+)\s+"#,
                                              r#"... bench:\s+(?P<ns>[0-9,]+)\s+ns/iter"#,
                                              r#"\s+\(\+/-\s+(?P<variance>[0-9,]+)\)"#,
                                              r#"(?:\s+=\s+(?P<throughput>[0-9,]+))?"#);

impl Benchmark {
    /// Parses a single benchmark line into a Benchmark.
    fn parse(line: String) -> Option<Benchmark> {
        lazy_static! {
            static ref RE: Regex = Regex::new(BENCHMARK_REGEX).unwrap();
        }
        RE.captures(line.as_str()).map(|c| {
            fn drop_commas_and_parse(s: &str) -> Option<usize> {
                drop_commas(s).parse::<usize>().ok()
            }
            Benchmark {
                name: c.name("name").unwrap().into(),
                ns: c.name("ns").and_then(drop_commas_and_parse).unwrap(),
                variance: c.name("variance").and_then(drop_commas_and_parse).unwrap(),
                throughput: c.name("throughput").map(|t| drop_commas_and_parse(t).unwrap()),
            }
        })
    }

    /// Compares an old benchmark (self) with a new benchmark.
    fn compare(&self, new: &Self) -> Comparison {
        let diff_ns = new.ns as i64 - self.ns as i64;
        Comparison {
            diff_ns: diff_ns,
            diff_ratio: diff_ns as f64 / self.ns as f64,
        }
    }
}

/// A comparison between an old and a new benchmark.
/// All differences are reported in terms of measuring improvements
/// (negative) or regressions (positive). That is, if an old benchmark
/// is slower than a new benchmark, then the difference is negative.
/// Conversely, if an old benchmark is faster than a new benchmark,
/// then the difference is positive.
#[derive(Debug, Default)]
struct Comparison {
    diff_ns: i64,
    diff_ratio: f64,
}

macro_rules! io_err {
    ($e:expr) => {Err(io::Error::new(io::ErrorKind::Other, $e))}
}

fn drop_commas(s: &str) -> String {
    s.chars()
        .filter(|&b| b != ',')
        .collect::<String>()
}

fn parse_benchmarks(all_benchmarks: File) -> Box<Iterator<Item = Benchmark>> {
    let reader = BufReader::new(all_benchmarks);

    let lines = reader.lines().skip_while(|r| match *r {
        Ok(ref s) => s.is_empty(),
        _ => true,
    });

    Box::new(lines.filter_map(Result::ok)
        .filter_map(|line: String| Benchmark::parse(line)))
}

/// Takes two *sorted* vectors and a comparison function
/// Gives back a tuple of vectors:
///  - one for the elements unique to the first vector
///  - one for the pairs of elements found equal
///  - one of the elements unique to the second vector
fn find_overlap<F, T>(mut left: Vec<T>,
                      mut right: Vec<T>,
                      mut fun: F)
                      -> (Vec<T>, Vec<(T, T)>, Vec<T>)
    where F: FnMut(&T, &T) -> Ordering
{
    let mut res_left = Vec::new();
    let mut res_right = Vec::new();
    let mut overlap = Vec::new();

    loop {
        match (left.pop(), right.pop()) {
            (Some(left_item), Some(right_item)) => {
                // sorted from small to large but pop takes from the end (large) side!
                match fun(&right_item, &left_item) {
                    Less => {
                        res_left.push(left_item);
                        right.push(right_item);
                    }
                    Equal => overlap.push((left_item, right_item)),
                    Greater => {
                        res_right.push(right_item);
                        left.push(left_item);
                    }
                }
            }
            (None, Some(right_item)) => res_right.push(right_item),
            (Some(left_item), None) => res_left.push(left_item),
            (None, None) => break,
        }
    }

    (res_left, overlap, res_right)
}

// The following code has been picked from the Rust programming language main repository:
// https://github.com/rust-lang/rust/blob/20183f498fbd8465859bf47611e1165768b9cc59/src/libtest/lib.rs#L664-L686
// To comply with the license of the code, the license is copied here. It only applies to the
//  function `fmt_thousands_sep`.
//
// Copyright (c) 2010 The Rust Project Developers
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//
// Format a number with thousands separators
fn fmt_thousands_sep(mut n: usize, sep: char) -> String {
    use std::fmt::Write;
    let mut output = String::new();
    let mut trailing = false;
    for &pow in &[9, 6, 3, 0] {
        let base = 10_usize.pow(pow);
        if pow == 0 || trailing || n / base != 0 {
            if !trailing {
                output.write_fmt(format_args!("{}", n / base)).unwrap();
            } else {
                output.write_fmt(format_args!("{:03}", n / base)).unwrap();
            }
            if pow != 0 {
                output.push(sep);
            }
            trailing = true;
        }
        n %= base;
    }

    output
}

macro_rules! err_println {
    ($fmt:expr) => (err_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (err_print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! err_print {
    ($($arg:tt)*) => (io::stderr().write_fmt(format_args!($($arg)*)).unwrap(););
}

fn main() {
    use ShowOption::*;

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let fst_replace: Box<Fn(&str) -> String> = match args.flag_strip_fst {
        None => Box::new(|s: &str| String::from(s)),
        Some(s) => {
            match Regex::new(s.as_str()) {
                Ok(re) => Box::new(move |s: &str| re.replace(s, "")),
                Err(e) => {
                    err_println!("ERROR: strip_fst: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };
    let snd_replace: Box<Fn(&str) -> String> = match args.flag_strip_snd {
        None => Box::new(|s: &str| String::from(s)),
        Some(s) => {
            match Regex::new(s.as_str()) {
                Ok(re) => Box::new(move |s: &str| re.replace(s, "")),
                Err(e) => {
                    err_println!("ERROR: strip_fst: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    let (mut fst, mut snd) = match args.arg_name {
        None => {
            let fst = parse_benchmarks(File::open(&args.arg_file[0]).unwrap());
            let snd = parse_benchmarks(File::open(&args.arg_file[1]).unwrap());

            let fst = fst.map(|mut b| { b.name = fst_replace(b.name.as_str()); b }).collect::<Vec<Benchmark>>();
            let snd = snd.map(|mut b| { b.name = snd_replace(b.name.as_str()); b }).collect::<Vec<Benchmark>>();

            (fst, snd)
        }
        Some(ref arg_names) => {
            let name_fst = &arg_names[0];
            let name_snd = &arg_names[1];

            let parse_file = |s| parse_benchmarks(File::open(s).unwrap()).into_iter();

            let benchmarks = args.arg_file.iter().flat_map(parse_file);

            let mut fst = Vec::new();
            let mut snd = Vec::new();

            for mut b in benchmarks {
                // explicitly moving the name out of b here so it can be assigned later
                let name = b.name;
                let mut split = name.splitn(2, "::");
                // There should always be a string here
                let implementation = split.next().unwrap();
                // But there may not be a :: in the string, so the second part may not exist
                if let Some(test) = split.next() {
                    if implementation == name_fst {
                        b.name = fst_replace(test);
                        fst.push(b);
                    } else if implementation == name_snd {
                        b.name = snd_replace(test);
                        snd.push(b);
                    }
                }
            }

            (fst, snd)
        }
    };

    fst.sort_by(|b1, b2| b1.name.cmp(&b2.name));
    snd.sort_by(|b1, b2| b1.name.cmp(&b2.name));

    let (missed_fst, overlap, missed_snd) = find_overlap(fst, snd, |o, n| o.name.cmp(&n.name));

    if !missed_fst.is_empty() {
        err_println!("WARNING: benchmarks present in fst but not in snd: {:?}",
                     missed_fst.into_iter()
                         .map(|o| o.name)
                         .collect::<Vec<String>>());
    }
    if !missed_snd.is_empty() {
        err_println!("WARNING: benchmarks present in snd but not in fst: {:?}",
                     missed_snd.into_iter()
                         .map(|n| n.name)
                         .collect::<Vec<String>>());
    }

    let mut output = TabWriter::new(io::stdout());

    match args.arg_name {
        Some(arg_names) => {
            write!(output,
                   "name\t{} ns/iter\t{} ns/iter\tdiff ns/iter\tdiff %\n",
                   arg_names[0],
                   arg_names[1])
                .unwrap()
        }
        None => {
            write!(output,
                   "name\t{} ns/iter\t{} ns/iter\tdiff ns/iter\tdiff %\n",
                   args.arg_file[0],
                   args.arg_file[1])
                .unwrap()
        }
    }

    for (fst, snd) in overlap {
        let comparison = fst.compare(&snd);
        let name = fst.name;
        let percentage = comparison.diff_ratio * 100f64;
        if args.flag_threshold.map_or(false, |threshold| percentage.abs() < threshold as f64) {
            continue;
        }
        match args.flag_show {
            Regressions => {
                if comparison.diff_ns <= 0 {
                    continue;
                }
            }
            Improvements => {
                if comparison.diff_ns >= 0 {
                    continue;
                }
            }
            Both => {}
        }

        write!(output, "{}\t", name).unwrap();
        write!(output, "{}", fmt_thousands_sep(fst.ns, ',')).unwrap();
        if args.flag_variance {
            write!(output, " (+/- {})", fst.variance).unwrap();
        }
        write!(output, "\t").unwrap();
        write!(output, "{}", fmt_thousands_sep(snd.ns, ',')).unwrap();
        if args.flag_variance {
            write!(output, " (+/- {})", snd.variance).unwrap();
        }
        write!(output, "\t").unwrap();
        if comparison.diff_ns < 0 {
            write!(output, "-").unwrap();
        }
        write!(output,
               "{}\t",
               fmt_thousands_sep(comparison.diff_ns.abs() as usize, ','))
            .unwrap();
        write!(output, "{:.2}%\n", percentage).unwrap();
    }

    output.flush().unwrap();
}
