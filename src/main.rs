extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate prettytable;

use std::io::{self, BufRead, Read};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process;

use docopt::Docopt;
use prettytable::Table;
use prettytable::format;

use benchmark::{Benchmarks, Benchmark};
use error::{Result, Error};

mod benchmark;
mod error;

macro_rules! eprintln {
    ($($tt:tt)*) => {{
        use std::io::{Write, stderr};
        writeln!(&mut stderr(), $($tt)*).unwrap();
    }}
}

const USAGE: &'static str = r#"
Compares Rust micro-benchmark results.

Usage:
    cargo benchcmp [options] <old> <new>
    cargo benchcmp [options] <old> <new> <file>
    cargo benchcmp -h | --help
    cargo benchcmp --version

The first version takes two files and compares the common benchmarks.

The second version takes two benchmark name prefixes and one benchmark output
file, and compares the common benchmarks (as determined by comparing the
benchmark names with their prefixes stripped). Benchmarks not matching either
prefix are ignored completely.

If benchmark output is sent on stdin, then the second version is used and the
third file parameter is not needed.

Options:
    -h, --help           Show this help message and exit.
    --version            Show the version.
    --threshold <n>      Show only comparisons with a percentage change greater
                         than this threshold.
    --variance           Show the variance of each benchmark.
    --improvements       Show only improvements.
    --regressions        Show only regressions.
    --color <when>       Show colored rows: never, always or auto [default: auto]
"#;

#[derive(Debug, RustcDecodable)]
struct Args {
    arg_old: String,
    arg_new: String,
    arg_file: Option<String>,
    flag_threshold: Option<u8>,
    flag_variance: bool,
    flag_improvements: bool,
    flag_regressions: bool,
    flag_color: When,
}

#[derive(Debug, RustcDecodable)]
enum When {
    Never, Always, Auto
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.version(Some(version())).decode())
        .unwrap_or_else(|e| e.exit());
    if let Err(e) = args.run() {
        eprintln!("{}", e);
        process::exit(1);
    }
}

impl Args {
    fn run(&self) -> Result<()> {
        let (name_old, name_new) = self.names();
        let benches = try!(self.parse_benchmarks()).paired();
        let mut output = Table::new();
        output.set_format(*format::consts::FORMAT_CLEAN);
        output.add_row(row![
            b->"name",
            b->format!("{} ns/iter", name_old),
            b->format!("{} ns/iter", name_new),
            br->"diff ns/iter",
            br->"diff %"
        ]);
        for c in benches.comparisons() {
            let abs_per = (c.diff_ratio * 100f64).abs().trunc() as u8;
            let regression = c.diff_ns < 0;
            if self.flag_threshold.map_or(false, |t| abs_per < t)
                || self.flag_regressions && regression
                || self.flag_improvements && !regression {
                continue;
            }
            output.add_row(c.to_row(self.flag_variance, regression));
        }

        match self.flag_color {
            When::Auto => output.printstd(),
            When::Never => try!(output.print(&mut io::stdout())),
            When::Always => output.print_tty(true),
        }

        // If there were any unpaired benchmarks, show them now.
        if !benches.missing_old().is_empty() {
            let missed = benches
                .missing_old().iter().map(|b| b.name.to_string())
                .collect::<Vec<String>>().join(", ");
            eprintln!("WARNING: benchmarks in old but not in new: {}", missed);
        }
        if !benches.missing_new().is_empty() {
            let missed = benches
                .missing_new().iter().map(|b| b.name.to_string())
                .collect::<Vec<String>>().join(", ");
            eprintln!("WARNING: benchmarks in new but not in old: {}", missed);
        }
        Ok(())
    }

    /// Parse benchmarks from the command line invocation given.
    fn parse_benchmarks(&self) -> Result<Benchmarks> {
        if let Some(ref one_file) = self.arg_file {
            if one_file == "-" {
                let mut buf = String::new();
                let stdin = io::stdin();
                try!(stdin.lock().read_to_string(&mut buf));
                self.parse_buf_benchmarks(&buf)
            } else {
                self.parse_file_benchmarks(one_file)
            }
        } else {
            self.parse_old_new_benchmarks()
        }
    }

    /// Parses benchmarks from two files: one containing old benchmark output
    /// and another containing new benchmark output.
    fn parse_old_new_benchmarks(&self) -> Result<Benchmarks> {
        let bold = io::BufReader::new(try!(open_file(&self.arg_old)));
        let bnew = io::BufReader::new(try!(open_file(&self.arg_new)));

        let mut benches = Benchmarks::new();
        for line in bold.lines() {
            let line = try!(line);
            if let Ok(bench) = line.parse() {
                benches.add_old(bench);
            }
        }
        for line in bnew.lines() {
            let line = try!(line);
            if let Ok(bench) = line.parse() {
                benches.add_new(bench);
            }
        }
        Ok(benches)
    }

    /// Parses benchmarks from one file with two prefixes. The first prefix
    /// identifies benchmarks in the old set and the second prefix identifies
    /// benchmarks in the new set where all benchmarks are found in one file.
    fn parse_file_benchmarks<P>(
        &self,
        file: P,
    ) -> Result<Benchmarks>
    where P: AsRef<Path> {
        // Slurp up the entire file so that we can reuse this code with the
        // code for reading benchmarks on stdin.
        let mut buf = String::new();
        try!(try!(File::open(file)).read_to_string(&mut buf));
        self.parse_buf_benchmarks(&buf)
    }

    /// Same as parse_file_benchmarks, but straight from the buffer.
    fn parse_buf_benchmarks(&self, buf: &str) -> Result<Benchmarks> {
        let mut benches = Benchmarks::new();
        for line in buf.lines() {
            let mut bench: Benchmark = match line.parse() {
                Err(_) => continue,
                Ok(bench) => bench,
            };
            if bench.name.starts_with(&self.arg_old) {
                bench.name = bench.name[self.arg_old.len()..].to_string();
                benches.add_old(bench);
            } else if bench.name.starts_with(&self.arg_new) {
                bench.name = bench.name[self.arg_new.len()..].to_string();
                benches.add_new(bench);
            }
        }
        Ok(benches)
    }

    /// Returns the names that should be used in the column header.
    fn names(&self) -> (String, String) {
        // If either of the names are empty, substitute them with defaults.
        let arg_old =
            if self.arg_old.is_empty() {
                "old".to_string()
            } else {
                self.arg_old.to_string()
            };
        let arg_new =
            if self.arg_new.is_empty() {
                "new".to_string()
            } else {
                self.arg_new.to_string()
            };
        // The names could be either in the prefixes or in the file paths.
        let (old, new) = (Path::new(&arg_old), Path::new(&arg_new));
        // No files paths? Don't do anything smart.
        if old.iter().count() <= 1 || new.iter().count() <= 1 {
            return (arg_old.clone(), arg_new.clone());
        }
        // If we have file paths, try to find the shortest string that
        // differentiates them.
        let (mut uold, mut unew) = (vec![], vec![]);
        for (o, n) in old.iter().rev().zip(new.iter().rev()) {
            uold.push(o.to_string_lossy().into_owned());
            unew.push(n.to_string_lossy().into_owned());
            if o != n {
                break;
            }
        }
        // If for some reason one of these is empty, just fall back to the
        // names given.
        if uold.is_empty() || unew.is_empty() {
            return (arg_old.clone(), arg_new.clone());
        }
        uold.reverse();
        unew.reverse();
        let pold: PathBuf = uold.into_iter().collect();
        let pnew: PathBuf = unew.into_iter().collect();
        (pold.display().to_string(), pnew.display().to_string())
    }
}

fn version() -> String {
    let (maj, min, pat) = (
        option_env!("CARGO_PKG_VERSION_MAJOR"),
        option_env!("CARGO_PKG_VERSION_MINOR"),
        option_env!("CARGO_PKG_VERSION_PATCH"),
    );
    match (maj, min, pat) {
        (Some(maj), Some(min), Some(pat)) =>
            format!("{}.{}.{}", maj, min, pat),
        _ => "".to_owned(),
    }
}

/// `open_file` is like `File::open`, except it gives a better error message
/// when it fails (i.e., it includes the file path).
fn open_file<P: AsRef<Path>>(path: P) -> Result<File> {
    File::open(&path).map_err(|err| {
        Error::OpenFile {
            path: path.as_ref().to_path_buf(),
            err: err,
        }
    })
}
