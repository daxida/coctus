mod test_result;

use std::io::Write;
use std::process::Command;
use std::time::Duration;

use test_result::CommandExit;
pub use test_result::TestResult;
use wait_timeout::ChildExt;

use crate::clash::Testcase;

/// Run a command against testcases one at a time.
///
/// # Examples
///
/// ```
/// use clashlib::clash::Testcase;
/// use clashlib::solution::lazy_run;
///
/// let testcases = [
///     Testcase {
///         index: 1,
///         title: String::from("Test #1"),
///         test_in: String::from("hey"),
///         test_out: String::from("hey"),
///         is_validator: false,
///     }
/// ];
/// let mut command = std::process::Command::new("cat");
/// let timeout = std::time::Duration::from_secs(5);
///
/// for (testcase, test_result) in lazy_run(&testcases, &mut command, &timeout) {
///     assert_eq!(testcase.title, "Test #1");
///     assert!(test_result.is_success());
/// }
/// ```
pub fn lazy_run<'a>(
    testcases: impl IntoIterator<Item = &'a Testcase>,
    run_command: &'a mut Command,
    timeout: &'a Duration,
) -> impl IntoIterator<Item = (&'a Testcase, TestResult)> {
    testcases.into_iter().map(|test| {
        let result = run_testcase(test, run_command, timeout);
        (test, result)
    })
}

/// Run a command against a single testcase.
pub fn run_testcase(testcase: &Testcase, run_command: &mut Command, timeout: &Duration) -> TestResult {
    let mut run = match run_command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(run) => run,
        Err(error) => {
            let program = run_command.get_program().to_str().unwrap_or("Unable to run command");
            let error_msg = format!("{}: {}", program, error);
            return TestResult::UnableToRun { error_msg }
        }
    };

    run.stdin
        .as_mut()
        .expect("STDIN of child process should be captured")
        .write_all(testcase.test_in.as_bytes())
        .expect("STDIN of child process should be writable");

    let timed_out = run
        .wait_timeout(*timeout)
        .expect("Process should be able to wait for execution")
        .is_none();

    if timed_out {
        run.kill().expect("Process should have been killed");
    }

    let output = run.wait_with_output().expect("Process should allow waiting for its execution");

    let exit_status = if timed_out {
        CommandExit::Timeout
    } else if output.status.success() {
        CommandExit::Ok
    } else {
        CommandExit::Error
    };
    TestResult::from_output(&testcase.test_out, output.stdout, output.stderr, exit_status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passing_solution() {
        let clash = crate::test_helper::sample_puzzle("stub_and_solution_tester").unwrap();
        let mut run_cmd = Command::new("tr");
        run_cmd.arg("X");
        run_cmd.arg("b");
        let timeout = Duration::from_secs(1);
        assert!(lazy_run(clash.testcases(), &mut run_cmd, &timeout)
            .into_iter()
            .all(|(_, test_result)| test_result.is_success()))
    }

    #[test]
    fn test_failing_solution() {
        let clash = crate::test_helper::sample_puzzle("stub_and_solution_tester").unwrap();
        let timeout = Duration::from_secs(1);
        let mut run_cmd = Command::new("cat");
        assert!(lazy_run(clash.testcases(), &mut run_cmd, &timeout)
            .into_iter()
            .all(|(_, test_result)| !test_result.is_success()))
    }
}
