extern crate second_law;

macro_rules! new_scene {
    () => ({
        use second_law;
        if cfg!(target_os = "windows") {
            second_law::Scene::new(format!("{}.exe", env!("CARGO_PKG_NAME")))
        } else {
            second_law::Scene::new(env!("CARGO_PKG_NAME"))
        }
    });
}

fn new_ucmd() -> second_law::UCommand {
    let mut scene: second_law::Scene = new_scene!();
    scene.subcmd_arg("benchcmp");
    scene.ucmd()
}

#[test]
fn invalid_arguments() {
    let mut ucmd: second_law::UCommand = new_scene!().ucmd();
    ucmd.fails().no_stdout().stderr_is_fixture("invalid_arguments.expected");
}

#[test]
fn version() {
    new_ucmd().arg("--version").succeeds().no_stderr().stdout_only(env!("CARGO_PKG_VERSION"));
}

#[test]
fn same_input() {
    new_ucmd().args(&["bench_output_1.txt", "bench_output_1.txt"]).succeeds().stdout_is_fixture("same_input.expected");
}

#[test]
fn different_input() {
    new_ucmd()
        .args(&["bench_output_2.txt", "bench_output_3.txt"])
        .succeeds()
        .no_stderr()
        .stdout_is_fixture("different_input.expected");
}

#[test]
fn non_overlapping_input() {
    new_ucmd()
        .args(&["bench_output_1.txt", "bench_output_2.txt"])
        .succeeds()
        .stderr_is_fixture("non_overlapping_input.expected")
        .no_stdout();
}

#[cfg(unix)]
#[test]
fn different_input_colored() {
    let mut scene: second_law::Scene = new_scene!();
    scene.subcmd_arg("benchcmp");
    // NOTE: keeping the environment here so that terminfo is available,
    //  which is required to get colour code in the output
    scene.ucmd_keepenv()
        .args(&["--color", "always", "bench_output_2.txt", "bench_output_3.txt"])
        .succeeds()
        .no_stderr()
        .stdout_is_fixture("different_input_colored.expected");
}

#[test]
fn different_input_selections() {
    new_ucmd()
        .args(&["dense::", "dense_boxed::", "bench_output_1.txt"])
        .succeeds()
        .no_stderr()
        .stdout_is_fixture("different_input_selections.expected");
}

#[test]
fn stdin() {
    new_ucmd()
        .args(&["dense::", "dense_boxed::", "-"])
        .pipe_in_fixture("bench_output_1.txt")
        .succeeds()
        .no_stderr()
        .stdout_is_fixture("different_input_selections.expected");
}

#[test]
fn empty_results() {
    new_ucmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--regressions", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("empty_results.expected");
}

#[test]
fn within_threshold_1_comparing_4_5() {
    new_ucmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--threshold", "1"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("4_cmp_5_within_threshold.expected");
}

#[test]
fn within_threshold_12_comparing_6_7() {
    new_ucmd()
        .args(&["bench_output_6.txt", "bench_output_7.txt", "--threshold", "12"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("6_cmp_7_within_threshold.expected");
}

#[test]
fn within_threshold_3_comparing_6_7_improvements() {
    new_ucmd()
        .args(&["bench_output_6.txt", "bench_output_7.txt", "--threshold", "3", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("6_cmp_7_within_threshold_improvements.expected");
}

#[test]
fn within_threshold_4_comparing_6_7_regressions() {
    new_ucmd()
        .args(&["bench_output_6.txt", "bench_output_7.txt", "--threshold", "4", "--regressions"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("6_cmp_7_within_threshold_regressions.expected");
}

#[test]
fn zero_regressions() {
    new_ucmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--regressions"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("zero_regressions.expected");
}

#[test]
fn zero_regressions_threshold() {
    new_ucmd()
        .args(&["bench_output_4.txt", "bench_output_5.txt", "--threshold", "2", "--regressions"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("zero_regressions.expected");
}

#[test]
fn zero_improvements() {
    new_ucmd()
        .args(&["bench_output_4.txt", "bench_output_8.txt", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("zero_improvements.expected");
}

#[test]
fn zero_improvements_threshold() {
    new_ucmd()
        .args(&["bench_output_4.txt", "bench_output_8.txt", "--threshold", "2", "--improvements"])
        .succeeds()
        .no_stdout()
        .stderr_is_fixture("zero_improvements.expected");
}
