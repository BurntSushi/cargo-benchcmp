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

static BENCH_OUTPUT_1: &'static str = include_str!("fixtures/bench_output_1.txt");

#[cfg(not(windows))]
static SAME_INPUT: &'static str = include_str!("fixtures/same_input.expected");
#[cfg(not(windows))]
static DIFFERENT_INPUT: &'static str = include_str!("fixtures/different_input.expected");
#[cfg(not(windows))]
static DIFFERENT_INPUT_SELECTIONS: &'static str = include_str!("fixtures/different_input_selections\
                                                                .expected");

#[cfg(windows)]
static SAME_INPUT: &'static str = include_str!("fixtures/same_input.windows.expected");
#[cfg(windows)]
static DIFFERENT_INPUT: &'static str = include_str!("fixtures/different_input.windows.expected");
#[cfg(windows)]
static DIFFERENT_INPUT_SELECTIONS: &'static str = include_str!("fixtures/different_input_selections\
                                                                .windows.expected");

fn new_ucmd() -> second_law::UCommand {
    let mut scene: second_law::Scene = new_scene!();
    scene.subcmd_arg("benchcmp");
    scene.ucmd()
}

#[test]
fn invalid_arguments() {
    let mut ucmd: second_law::UCommand = new_scene!().ucmd();
    ucmd.fails().stderr_is_fixture("invalid_arguments.expected");
}

#[test]
fn version() {
    new_ucmd().arg("--version").succeeds().stdout_only(env!("CARGO_PKG_VERSION"));
}

#[test]
fn help() {
    for same_arg in &["-h", "--help"] {
        new_ucmd().arg(same_arg).succeeds().stdout_is_fixture("usage.expected");
    }
}

#[test]
fn same_input() {
    new_ucmd().args(&["bench_output_1.txt", "bench_output_1.txt"]).succeeds().stdout_is(SAME_INPUT);
}

#[test]
fn different_input() {
    new_ucmd()
        .args(&["bench_output_2.txt", "bench_output_3.txt"])
        .succeeds()
        .stdout_is(DIFFERENT_INPUT);
}

// TODO: Add tests with inputs with non-overlapping names and check stderr for the warnings

#[test]
fn different_input_selections() {
    new_ucmd()
        .args(&["dense::", "dense_boxed::", "bench_output_1.txt"])
        .succeeds()
        .stdout_is(DIFFERENT_INPUT_SELECTIONS);
}

#[test]
fn stdin() {
    new_ucmd()
        .args(&["dense::", "dense_boxed::", "-"])
        .pipe_in(BENCH_OUTPUT_1)
        .succeeds()
        .stdout_is(DIFFERENT_INPUT_SELECTIONS);
}
