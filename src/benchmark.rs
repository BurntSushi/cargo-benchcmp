use std::cmp;
use std::str::FromStr;

use prettytable::row::Row;
use regex::Regex;

/// Two sets of benchmarks that are comparable but haven't been paired up yet.
#[derive(Clone, Debug)]
pub struct Benchmarks {
    old: Vec<Benchmark>,
    new: Vec<Benchmark>,
}

impl Benchmarks {
    /// Create a new empty set of comparable benchmarks.
    pub fn from(pair: (Vec<Benchmark>, Vec<Benchmark>)) -> Benchmarks {
        Benchmarks {
            old: pair.0,
            new: pair.1,
        }
    }

    /// Create a set of pairwise comparisons between benchmarks.
    ///
    /// The old and new benchmarks are paired based on whether they have
    /// equivalent names. Benchmarks without a pair are marked as unpaired.
    pub fn paired(self) -> PairedBenchmarks {
        PairedBenchmarks::from(self)
    }
}

/// `PairedBenchmarks` is a set of paired benchmarks.
///
/// This also provides access to unpaired benchmarks.
#[derive(Clone, Debug)]
pub struct PairedBenchmarks {
    cmps: Vec<Comparison>,
    failed: Vec<Benchmark>,
    benched_new: Vec<Benchmark>,
    unpaired_old: Vec<Benchmark>,
    unpaired_new: Vec<Benchmark>,
}

impl From<Benchmarks> for PairedBenchmarks {
    fn from(mut benches: Benchmarks) -> PairedBenchmarks {
        benches.old.sort();
        benches.new.sort();
        let ov = Overlap::find(benches.old, benches.new, Benchmark::cmp);

        let (failed, benched): (Vec<(Benchmark, Benchmark)>, _)
            = ov.overlap.into_iter().partition(|(_o, n)| n.failed_msg.is_some());
        let (benched_new, overlap): (Vec<(Benchmark, Benchmark)>, _)
            = benched.into_iter().partition(|(o, _n)| o.failed_msg.is_some());

        let cmps = overlap.into_iter().map(|(a, b)| a.compare(b)).collect();
        PairedBenchmarks {
            cmps: cmps,
            failed: failed.into_iter().map(|(_, n)|n).collect(),
            benched_new: benched_new.into_iter().map(|(_, n)|n).collect(),
            unpaired_old: ov.left,
            unpaired_new: ov.right,
        }
    }
}

impl PairedBenchmarks {
    /// Returns all pairwise benchmark comparisons.
    ///
    /// Each comparison provides access to the old and new benchmarks.
    pub fn comparisons(&self) -> &[Comparison] {
        &self.cmps
    }

    /// Returns all benchmarks that were in the old set that were not found
    /// in the new set.
    pub fn missing_old(&self) -> &[Benchmark] {
        &self.unpaired_old
    }

    /// Returns all benchmarks that were in the new set that were not found
    /// in the old set.
    pub fn missing_new(&self) -> &[Benchmark] {
        &self.unpaired_new
    }

    /// Returns all benchmarks that were failed in the new set that were passed
    /// in the old set.
    pub fn failures(&self) -> &[Benchmark] {
        // old: _, new: FAILED
        &self.failed
    }

    /// Returns all benchmarks that were passed in the new set that were failed
    /// in the old set.
    pub fn new_benchmarks(&self) -> &[Benchmark] {
        // old: FAILED, new: benched
        &self.benched_new
    }
}

/// All extractable data from a single micro-benchmark.
#[derive(Default, Clone, Debug)]
pub struct Benchmark {
    pub name: String,
    pub ns: u64,
    pub variance: u64,
    pub throughput: Option<u64>,
    pub failed_msg: Option<FailedMsg>,
}

impl Eq for Benchmark {}

impl PartialEq for Benchmark {
    fn eq(&self, other: &Benchmark) -> bool {
        self.name == other.name
    }
}

impl Ord for Benchmark {
    fn cmp(&self, other: &Benchmark) -> cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl PartialOrd for Benchmark {
    fn partial_cmp(&self, other: &Benchmark) -> Option<cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

lazy_static! {
    static ref BENCHMARK_REGEX: Regex = Regex::new(r##"(?x)
        test\s+(?P<name>\S+)                        # test   mod::test_name
        \s+...\sbench:\s+(?P<ns>[0-9,]+)\s+ns/iter  # ... bench: 1234 ns/iter
        \s+\(\+/-\s+(?P<variance>[0-9,]+)\)         # (+/- 4321)
        (?:\s+=\s+(?P<throughput>[0-9,]+)\sMB/s)?   # =   2314 MB/s
    "##).unwrap();

    static ref BENCHMARK_REGEX_FAILED: Regex = Regex::new(r##"(?x)
        test\s+(?P<name>\S+)                        # test   mod::test_name
        \s+...\sFAILED                              # ... FAILED
    "##).unwrap();

    static ref BENCHMARK_REGEX_FAILED_MESSAGE1: Regex = Regex::new(r##"(?x)
        ----\s(?P<name>\S+)\sstdout\s----           # ---- bench::it_works stdout ----
    "##).unwrap();

    static ref BENCHMARK_REGEX_FAILED_MESSAGE2: Regex = Regex::new(r##"(?x)
        thread\s'(?P<thread>\S+)'                   # thread 'main'
        \s+panicked\sat\s'(?P<msg>.+)'              # panicked at 'called `Option::unwrap()` on a `None` value'
    "##).unwrap();
}

impl FromStr for Benchmark {
    type Err = ();

    /// Parses a single benchmark line into a Benchmark.
    fn from_str(line: &str) -> Result<Benchmark, ()> {
        if let Some(caps) = BENCHMARK_REGEX_FAILED.captures(line) {
            Ok(Benchmark {
                name: caps["name"].to_string(),
                .. Default::default()
            })
        } else {
            let caps = match BENCHMARK_REGEX.captures(line) {
                None => return Err(()),
                Some(caps) => caps,
            };
            let ns = match parse_commas(&caps["ns"]) {
                None => return Err(()),
                Some(ns) => ns,
            };
            let variance = match parse_commas(&caps["variance"]) {
                None => return Err(()),
                Some(variance) => variance,
            };
            let throughput = caps.name("throughput").and_then(|m| parse_commas(m.as_str()));
            Ok(Benchmark {
                name: caps["name"].to_string(),
                ns: ns,
                variance: variance,
                throughput: throughput,
                failed_msg: None,
            })
        }
    }
}

impl Benchmark {
    /// Compares an old benchmark (self) with a new benchmark.
    pub fn compare(self, new: Benchmark) -> Comparison {
        let diff_ns = new.ns as i64 - self.ns as i64;
        let diff_ratio = diff_ns as f64 / self.ns as f64;
        let speedup = 1.0 / (1.0 + diff_ratio);
        Comparison {
            old: self,
            new: new,
            diff_ns: diff_ns,
            diff_ratio: diff_ratio,
            speedup: speedup,
        }
    }

    pub fn fmt_ns(&self, variance: bool) -> String {
        let mut res = commafy(self.ns);
        if variance {
            res = format!("{} (+/- {})", res, self.variance);
        }
        if let Some(throughput) = self.throughput {
            res = format!("{} ({} MB/s)", res, throughput);
        }
        res
    }
}

/// Error message struct that all failed test cases have.
#[derive(Clone, Debug)]
pub struct FailedMsg {
    pub name: String,
    pub msg: String,
}

pub struct FailedMsgBuilder {
    name: String,
}

impl FailedMsgBuilder {
    pub fn build(self, line: &str) -> Result<FailedMsg, ()> {
        match BENCHMARK_REGEX_FAILED_MESSAGE2.find(line) {
            Some(caps) => {
                Ok(FailedMsg{
                    name: self.name,
                    msg: caps.as_str().to_string(),
                })
            }
            None => Err(())
        }
    }
}

impl FromStr for FailedMsgBuilder {
    type Err = ();

    fn from_str(line: &str) -> Result<FailedMsgBuilder, ()> {
        match BENCHMARK_REGEX_FAILED_MESSAGE1.captures(line) {
            Some(caps) => Ok(FailedMsgBuilder{ name: caps["name"].to_string() }),
            None => Err(())
        }
    }
}

/// A comparison between an old and a new benchmark.
/// All differences are reported in terms of measuring improvements
/// (negative) or regressions (positive). That is, if an old benchmark
/// is slower than a new benchmark, then the difference is negative.
/// Conversely, if an old benchmark is faster than a new benchmark,
/// then the difference is positive.
#[derive(Clone, Debug)]
pub struct Comparison {
    pub old: Benchmark,
    pub new: Benchmark,
    pub diff_ns: i64,
    pub diff_ratio: f64,
    pub speedup: f64,
}

impl Comparison {
    /// Convert this comparison to a formatted row useful for printing.
    ///
    /// The columns of the row are as follows: the name of the benchmark being
    /// compared, the old measurement, the new measurement, the measurement
    /// difference and the percent measurement difference. Negative differences
    /// imply an improvement in performance from old to new.
    pub fn to_row(&self, variance: bool, regression: bool) -> Row {
        let name = &self.old.name;
        let fst_ns = self.old.fmt_ns(variance);
        let snd_ns = self.new.fmt_ns(variance);
        let diff_ratio = format!("{:.2}%", self.diff_ratio * 100f64);
        let speedup = format!("x {:.2}", self.speedup);
        let diff_ns = {
            let diff_ns = commafy(self.diff_ns.abs() as u64);
            if self.diff_ns < 0 {
                format!("-{}", diff_ns)
            } else {
                diff_ns
            }
        };
        if regression {
            row![Fr->name, Fr->fst_ns, Fr->snd_ns, rFr->diff_ns, rFr->diff_ratio, rFr->speedup]
        } else {
            row![Fg->name, Fg->fst_ns, Fg->snd_ns, rFg->diff_ns, rFg->diff_ratio, rFg->speedup]
        }
    }
}

/// Returns what's left of the left vector and right vector that doesn't
/// overlap, and the overlap as a vector of pairs
#[derive(Debug)]
struct Overlap<T> {
    left: Vec<T>,
    overlap: Vec<(T, T)>,
    right: Vec<T>,
}

impl<T> Overlap<T> {
    /// Takes two *sorted* vectors in *ascending* order and a comparison function.
    ///
    /// Gives back a tuple of vectors, preserving the original sort order:
    ///  - one for the elements unique to the first vector
    ///  - one for the pairs of elements found equal
    ///  - one of the elements unique to the second vector
    fn find<F>(mut left: Vec<T>, mut right: Vec<T>, mut fun: F) -> Overlap<T>
        where F: FnMut(&T, &T) -> cmp::Ordering
    {
        use std::cmp::Ordering::*;

        let (mut rleft, mut rright, mut overlap) = (vec![], vec![], vec![]);
        loop {
            match (left.pop(), right.pop()) {
                (None, None) => break,
                (None, Some(right_item)) => rright.push(right_item),
                (Some(left_item), None) => rleft.push(left_item),
                (Some(left_item), Some(right_item)) => {
                    // sorted from small to large but pop takes from the end!
                    match fun(&right_item, &left_item) {
                        Less => {
                            rleft.push(left_item);
                            right.push(right_item);
                        }
                        Equal => overlap.push((left_item, right_item)),
                        Greater => {
                            rright.push(right_item);
                            left.push(left_item);
                        }
                    }
                }
            }
        }

        // We built these in reverse, so reverse them to get original order.
        rleft.reverse();
        rright.reverse();
        overlap.reverse();
        Overlap {
            left: rleft,
            overlap: overlap,
            right: rright,
        }
    }
}

/// Drops all commas in a string and parses it as a unsigned integer
fn parse_commas(s: &str) -> Option<u64> {
    drop_commas(s).parse().ok()
}

/// Drops all commas in a string
fn drop_commas(s: &str) -> String {
    s.chars().filter(|&b| b != ',').collect()
}

/// Commafy a number as a string.
fn commafy(n: u64) -> String {
    let mut with_commas = vec![];
    let dits: Vec<u8> = n.to_string().into_bytes().into_iter().rev().collect();
    let mut dits = &*dits;
    loop {
        if dits.len() < 3 {
            with_commas.extend_from_slice(dits);
            break;
        }
        let piece = &dits[0..3];
        dits = &dits[3..];
        with_commas.extend_from_slice(piece);
        if piece.len() == 3 && !dits.is_empty() && dits[0] != b'-' {
            with_commas.push(b',');
        }
    }
    with_commas.reverse();
    String::from_utf8(with_commas).unwrap()
}

#[cfg(test)]
mod tests {
    mod overlap {
        use super::super::Overlap;

        quickcheck! {
            fn overlap_correct(left: Vec<usize>, right: Vec<usize>) -> bool {
                let mut left = left;
                let mut right = right;
                left.sort();
                right.sort();

                let overlap = Overlap::find(left.clone(), right.clone(), usize::cmp);

                for (l,r) in overlap.overlap {
                    if l != r {
                        return false;
                    }
                }
                true
            }

            fn result_from_vecs(left: Vec<usize>, right: Vec<usize>) -> bool {
                let mut left = left;
                let mut right = right;
                left.sort();
                right.sort();

                let overlap = Overlap::find(left.clone(), right.clone(), usize::cmp);

                let (ov_left, ov_right): (Vec<usize>, Vec<usize>) =
                    overlap.overlap.into_iter().unzip();

                let mut left_reconstructed: Vec<usize> = overlap.left;
                left_reconstructed.extend(ov_left);
                left_reconstructed.sort();

                let mut right_reconstructed: Vec<usize> = overlap.right;
                right_reconstructed.extend(ov_right);
                right_reconstructed.sort();

                left == left_reconstructed && right == right_reconstructed
            }

            fn missing_correct(left: Vec<usize>, right: Vec<usize>) -> bool {
                let mut left = left;
                let mut right = right;
                left.sort();
                right.sort();

                // duplicates in either vec would make this check more complicated
                left.dedup();
                right.dedup();

                let overlap = Overlap::find(left.clone(), right.clone(), usize::cmp);

                for l in overlap.left {
                    if right.iter().find(|&&n| n == l).is_some() {
                        return false;
                    }
                }

                for r in overlap.right {
                    if left.iter().find(|&&n| n == r).is_some() {
                        return false;
                    }
                }

                true
            }
        }
    }

    mod commafy {
        use super::super::commafy;

        quickcheck! {
            fn comma_every_three(n: u64) -> bool {
                let commafied = commafy(n);
                let mut commafied = commafied.split(',');
                let s = commafied.next().unwrap();
                if s.len() == 0 || s.len() > 3 {
                    return false;
                }
                for s in commafied {
                    if s.len() != 3 {
                        return false;
                    }
                }
                true
            }

            fn number_matches(n: u64) -> bool {
                let commafied = commafy(n);
                let formatted = format!("{}", n);
                let stripped: String = commafied.chars().filter(|&b| b != ',').collect();
                formatted == stripped
            }
        }
    }

    mod benchmark {
        use super::super::Benchmark;
        use quickcheck::Arbitrary;
        use quickcheck::Gen;
        use rand::Rng;
        use rand::distributions::Alphanumeric;
        use std::iter;

        impl Arbitrary for Benchmark {
            fn arbitrary<G: Gen>(g: &mut G) -> Self {
                let (ns, variance, throughput): (u64, u64, Option<u64>) = Arbitrary::arbitrary(g);
                let name = {
                    let size = g.size();
                    let size = g.gen_range(1, size);
                    iter::repeat(()).map(|()| g.sample(Alphanumeric)).take(size).collect()
                };
                Benchmark {
                    name: name,
                    ns: ns,
                    variance: variance,
                    throughput: throughput,
                }
            }
        }

        fn deep_eq(b1: &Benchmark, b2: &Benchmark) -> bool {
            b1.name == b2.name && b1.variance == b2.variance && b1.ns == b2.ns &&
            b1.throughput == b2.throughput
        }

        fn as_string(b: &Benchmark) -> String {
            let res = format!("test {} ... bench: {} ns/iter (+/- {})",
                              b.name,
                              b.ns,
                              b.variance);
            if let Some(throughput) = b.throughput {
                format!("{} = {} MB/s", res, throughput)
            } else {
                res
            }
        }

        quickcheck! {
            fn reparse(b1: Benchmark) -> bool {
                if let Ok(b2) = as_string(&b1).parse() {
                    deep_eq(&b1, &b2)
                } else {
                    false
                }
            }
        }
    }
}
