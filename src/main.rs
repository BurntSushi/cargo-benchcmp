extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate lazy_static;
extern crate regex;
extern crate tabwriter;

mod benchmark;
mod utils;

use docopt::Docopt;
use regex::Regex;
use tabwriter::TabWriter;

use benchmark::{Benchmark, parse_benchmarks};
use utils::find_overlap;

use std::io;
use std::io::prelude::*;
use std::fs::File;

const USAGE: &'static str = r#"
Compare Rust micro-benchmarks by saving the output of the benchmark to file
and providing it into this command.
The first version takes two file and compares the common bench-tests.
The second version takes two names of implementations and one or more files,
and compares the common bench-tests of the two implementations based on the
name scheme "implementation::test".

Usage: cargo benchcmp [options] <file> <file>
       cargo benchcmp [options] <name> <name> [--] <file>...
       cargo benchcmp -h | --help


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

macro_rules! err_println {
    ($fmt:expr) => (err_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (err_print!(concat!($fmt, "\n"), $($arg)*));
}

macro_rules! err_print {
    ($($arg:tt)*) => (io::stderr().write_fmt(format_args!($($arg)*)).unwrap(););
}

macro_rules! create_replace_fn {
    ($e:expr, $s:expr) => { match $e {
        None => Box::new(|s: &str| String::from(s)),
        Some(s) => {
            match Regex::new(s.as_str()) {
                Ok(re) => Box::new(move |s: &str| re.replace(s, "")),
                Err(e) => {
                    err_println!(concat!("ERROR: strip_", $s, ": {}"), e);
                    return;
                }
            }
        }
    }}
}

// The create_replace_fn macro should use Box::new(String::from), but the compiler's type inference
//  can't handle it. 
#[allow(redundant_closure)]
fn main() {
    use ShowOption::*;

    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    let fst_replace: Box<Fn(&str) -> String> = create_replace_fn!(args.flag_strip_fst, "fst");
    let snd_replace: Box<Fn(&str) -> String> = create_replace_fn!(args.flag_strip_snd, "snd");

    let (mut fst, mut snd) = match args.arg_name {
        None => {
            let fst = parse_benchmarks(File::open(&args.arg_file[0]).unwrap());
            let snd = parse_benchmarks(File::open(&args.arg_file[1]).unwrap());

            let fst = fst.map(|mut b| {
                    b.name = fst_replace(b.name.as_str());
                    b
                })
                .collect::<Vec<Benchmark>>();
            let snd = snd.map(|mut b| {
                    b.name = snd_replace(b.name.as_str());
                    b
                })
                .collect::<Vec<Benchmark>>();

            (fst, snd)
        }
        Some(ref names) => {
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
                    if implementation == &names[0] {
                        b.name = fst_replace(test);
                        fst.push(b);
                    } else if implementation == &names[1] {
                        b.name = snd_replace(test);
                        snd.push(b);
                    }
                }
            }

            (fst, snd)
        }
    };

    let names = args.arg_name.map_or(args.arg_file, |a| a.to_vec());

    fst.sort_by(|b1, b2| b1.name.cmp(&b2.name));
    snd.sort_by(|b1, b2| b1.name.cmp(&b2.name));

    let (missed_fst, overlap, missed_snd) = find_overlap(fst, snd, |o, n| o.name.cmp(&n.name));

    warn_missing(missed_fst, "WARNING: benchmarks present in fst but not in snd: {:?}");
    warn_missing(missed_snd, "WARNING: benchmarks present in snd but not in fst: {:?}");

    let mut output = TabWriter::new(io::stdout());

    write!(output, "name\t{} ns/iter\t{} ns/iter\tdiff ns/iter\tdiff %\n",
           names[0],
           names[1]).unwrap();

    for comparison in overlap.into_iter().map(|(f, s)| f.compare(s)) {
        let trunc_abs_per = (comparison.diff_ratio * 100f64).abs().trunc() as u8;

        if args.flag_threshold.map_or(false, |threshold| trunc_abs_per < threshold) ||
           args.flag_show == Regressions && comparison.diff_ns <= 0 ||
           args.flag_show == Improvements && comparison.diff_ns >= 0 {
            continue;
        }

        comparison.write(&mut output, args.flag_variance).unwrap();
    }

    output.flush().unwrap();
}

fn warn_missing(v: Vec<Benchmark>, s: &str) {
    if !v.is_empty() {
        err_println!("{}: {:?}",
            s,
            v.into_iter()
                .map(|n| n.name)
                .collect::<Vec<String>>());
    }
}
