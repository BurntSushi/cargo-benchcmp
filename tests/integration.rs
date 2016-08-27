#[macro_use] extern crate second_law;

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
    new_ucmd().args(&["bench_output_1.txt", "bench_output_1.txt"]).succeeds().stdout_is_fixture("same_input.expected");
}

#[test]
fn different_input() {
    new_ucmd().args(&["bench_output_2.txt", "bench_output_3.txt"]).succeeds().stdout_is_fixture("different_input.expected");
}

// TODO: Add tests with inputs with non-overlapping names

#[test]
fn different_input_selections() {
    new_ucmd().args(&["dense::", "dense_boxed::", "bench_output_1.txt"]).succeeds().stdout_is_fixture("different_input_selections.expected");
}
