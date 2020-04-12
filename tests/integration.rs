use std::env;
use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

struct CommandUnderTest {
    raw: Command,
    stdin: Vec<u8>,
    run: bool,
    stdout: String,
    stderr: String,
}

impl CommandUnderTest {
    fn new() -> CommandUnderTest {
        // To find the directory where the built binary is, we walk up the directory tree of the test binary until the
        // parent is "target/".
        let mut binary_path = env::current_exe().expect("need current binary path to find binary to test");
        loop {
            {
                let parent = binary_path.parent();
                if parent.is_none() {
                    panic!("Failed to locate binary path from original path: {:?}", env::current_exe());
                }
                let parent = parent.unwrap();
                if parent.is_dir() && parent.file_name().unwrap() == "target" {
                    break;
                }
            }
            binary_path.pop();
        }

        binary_path.push(
            if cfg!(target_os = "windows") {
                format!("{}.exe", env!("CARGO_PKG_NAME"))
            } else {
                env!("CARGO_PKG_NAME").to_string()
            });

        let mut cmd = Command::new(binary_path);

        let mut work_dir = PathBuf::new();
        work_dir.push(env!("CARGO_MANIFEST_DIR"));
        work_dir.push("tests");
        work_dir.push("fixtures");

        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .current_dir(work_dir);

        CommandUnderTest {
            raw: cmd,
            run: false,
            stdin: Vec::new(),
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    fn keep_env(&mut self) -> &mut Self {
        self.raw.envs(env::vars());
        self
    }

    fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Self {
        self.raw.arg(arg);
        self
    }

    fn args<I, S>(&mut self, args: I) -> &mut Self
        where I: IntoIterator<Item=S>,
        S: AsRef<OsStr>
    {
        self.raw.args(args);
        self
    }

    fn pipe_in(&mut self, fixture: &str) -> &mut Self {
        self.stdin = Vec::from(fixture);
        self.raw.stdin(Stdio::piped());
        self
    }

    fn run(&mut self) -> ExitStatus {
        let mut child = self.raw.spawn().expect("failed to run command");

        if self.stdin.len() > 0 {
            let stdin = child.stdin.as_mut().expect("failed to open stdin");
            stdin.write_all(&self.stdin).expect("failed to write to stdin")
        }

        let output = child.wait_with_output().expect("failed waiting for command to complete");
        self.stdout = String::from_utf8(output.stdout).unwrap();
        self.stderr = String::from_utf8(output.stderr).unwrap();
        self.run = true;
        output.status
    }

    fn fails(&mut self) -> &mut Self {
        assert!(!self.run().success(), "expected command to fail");
        self
    }

    fn succeeds(&mut self) -> &mut Self {
        let status = self.run();
        assert!(status.success(), format!(
            "expected command to succeed, but it failed.\nexit code: {}\nstdout: {}\nstderr:{}\n",
            status.code().unwrap(),
            self.stdout,
            self.stderr,
        ));
        self
    }

    fn no_stdout(&mut self) -> &mut Self {
        assert!(self.run, "command has not yet been run, use succeeds()/fails()");
        assert!(self.stdout.is_empty(), format!("expected no stdout, got {}", self.stdout));
        self
    }

    fn no_stderr(&mut self) -> &mut Self {
        assert!(self.run, "command has not yet been run, use succeeds()/fails()");
        assert!(self.stderr.is_empty(), format!("expected no stderr, got {}", self.stderr));
        self
    }

    fn stdout_is(&mut self, expected: &str) -> &mut Self {
        assert!(self.run, "command has not yet been run, use succeeds()/fails()");
        assert_eq!(&self.stdout[..], expected, "stdout does not match expected");
        self
    }

    fn stderr_is(&mut self, expected: &str) -> &mut Self {
        assert!(self.run, "command has not yet been run, use succeeds()/fails()");
        assert_eq!(&self.stderr[..], expected, "stderr does not match expected");
        self
    }
}

fn new_cmd() -> CommandUnderTest {
    let mut cmd = CommandUnderTest::new();
    cmd.arg("benchcmp");
    cmd
}

#[test]
fn invalid_arguments() {
    let mut cmd = CommandUnderTest::new();
    cmd.fails().no_stdout().stderr_is(include_str!("fixtures/invalid_arguments.expected"));
}

#[test]
fn version() {
    new_cmd().arg("--version").succeeds().no_stderr().stdout_is(&format!("{}\n", env!("CARGO_PKG_VERSION")));
}

#[test]
fn same_input() {
    new_cmd()
        .args(&["bench_output_1.txt", "bench_output_1.txt"])
        .succeeds()
        .stdout_is(include_str!("fixtures/same_input.expected"));
}

#[test]
fn different_input() {
    new_cmd()
        .args(&["bench_output_2.txt", "bench_output_3.txt"])
        .succeeds()
        .no_stderr()
        .stdout_is(include_str!("fixtures/different_input.expected"));
}

#[test]
fn non_overlapping_input() {
    new_cmd()
        .args(&["bench_output_1.txt", "bench_output_2.txt"])
        .succeeds()
        .stderr_is(include_str!("fixtures/non_overlapping_input.expected"))
        .no_stdout();
}

#[cfg(unix)]
#[test]
fn different_input_colored() {
    new_cmd()
        // NOTE: keeping the environment here so that terminfo is available,
        //  which is required to get colour code in the output
        .keep_env()
        .args(&["--color", "always", "bench_output_2.txt", "bench_output_3.txt"])
        .succeeds()
        .no_stderr()
        .stdout_is(include_str!("fixtures/different_input_colored.expected"));
}

#[test]
fn different_input_selections() {
    new_cmd()
        .args(&["dense::", "dense_boxed::", "bench_output_1.txt"])
        .succeeds()
        .no_stderr()
        .stdout_is(include_str!("fixtures/different_input_selections.expected"));
}

#[test]
fn stdin() {
    new_cmd()
        .args(&["dense::", "dense_boxed::", "-"])
        .pipe_in(include_str!("fixtures/bench_output_1.txt"))
        .succeeds()
        .no_stderr()
        .stdout_is(include_str!("fixtures/different_input_selections.expected"));
}

#[test]
fn empty_results() {
    new_cmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--regressions", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/empty_results.expected"));
}

#[test]
fn within_threshold_1_comparing_4_5() {
    new_cmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--threshold", "1"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/4_cmp_5_within_threshold.expected"));
}

#[test]
fn within_threshold_12_comparing_6_7() {
    new_cmd()
        .args(&["bench_output_6.txt", "bench_output_7.txt", "--threshold", "12"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/6_cmp_7_within_threshold.expected"));
}

#[test]
fn within_threshold_3_comparing_6_7_improvements() {
    new_cmd()
        .args(&["bench_output_6.txt", "bench_output_7.txt", "--threshold", "3", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/6_cmp_7_within_threshold_improvements.expected"));
}

#[test]
fn within_threshold_4_comparing_6_7_regressions() {
    new_cmd()
        .args(&["bench_output_6.txt", "bench_output_7.txt", "--threshold", "4", "--regressions"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/6_cmp_7_within_threshold_regressions.expected"));
}

#[test]
fn zero_regressions() {
    new_cmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--regressions"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/zero_regressions.expected"));
}

#[test]
fn zero_regressions_threshold() {
    new_cmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--threshold", "2", "--regressions"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/zero_regressions.expected"));
}

#[test]
fn zero_improvements() {
    new_cmd()
        .args(&["bench_output_4.txt", "bench_output_8.txt", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/zero_improvements.expected"));
}

#[test]
fn zero_improvements_threshold() {
    new_cmd()
        .args(&["bench_output_4.txt", "bench_output_8.txt", "--threshold", "2", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is(include_str!("fixtures/zero_improvements.expected"));
}

#[test]
fn repeated_runs() {
    new_cmd()
        .args(&["bench_output_8.txt", "bench_output_8_repeated.txt", "--variance"])
        .succeeds()
        .no_stderr()
        .stdout_is(include_str!("fixtures/repeated_runs.expected"));
}
