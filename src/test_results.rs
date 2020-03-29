use crate::cli;
use crate::errors::RuntError;

/// Track the state of TestResult.
#[derive(Debug, PartialEq)]
pub enum TestState {
    /// The comparison succeeded.
    Correct,
    /// The .expect file is missing. Contains the generated expectation string.
    Missing(String),
    /// The comparison failed. Contains the the generated expectation string
    /// and the contents of the expect file.
    Mismatch(
        String, // Generated expect string.
        String, // Contents of the expect file.
    ),
}

/// Store information related to one test.
#[derive(Debug)]
pub struct TestResult {
    /// Path of the test
    pub path: std::path::PathBuf,

    /// Return status of the test.
    pub status: i32,

    /// STDOUT captured from the test.
    pub stdout: String,

    /// STRERR captured from the test.
    pub stderr: String,

    /// Result of comparison
    pub state: TestState,
}

impl TestResult {
    /// Save the results of the test suite into the expect file.
    pub fn save_results(&self) -> Result<(), RuntError> {
        use std::fs;
        use TestState as TS;
        match &self.state {
            TS::Correct => Ok(()),
            TS::Missing(expect) | TS::Mismatch(expect, _) => {
                Ok(fs::write(expect_file(&self.path), expect)?)
            }
        }
    }

    /// Generate colorized string to report the results of this test.
    pub fn report_str(&self, show_diff: bool) -> String {
        use crate::diff;
        use colored::*;
        use TestState as TS;

        let mut buf = String::new();
        let path_str = self.path.to_str().unwrap();
        match &self.state {
            TS::Missing(expect_string) => {
                buf.push_str(&"⚬ miss - ".yellow().to_string());
                buf.push_str(&path_str.yellow().to_string());
                if show_diff {
                    buf.push_str("\n");
                    buf.push_str(&expect_string);
                }
            }
            TS::Correct => {
                buf.push_str(&"⚬ pass - ".green().to_string());
                buf.push_str(&path_str.green().to_string());
            }
            TS::Mismatch(expect_string, contents) => {
                buf.push_str(&"⚬ fail - ".red().to_string());
                buf.push_str(&path_str.red().to_string());
                if show_diff {
                    let diff = diff::gen_diff(&contents, &expect_string);
                    buf.push_str("\n");
                    buf.push_str(&diff);
                }
            }
        };
        buf.to_string()
    }
}

/// Result of running a TestSuite.
pub struct TestSuiteResult(pub String, pub Vec<TestResult>, pub Vec<RuntError>);

impl TestSuiteResult {
    pub fn only_results(mut self, only: &Option<cli::OnlyOpt>) -> Self {
        use cli::OnlyOpt as O;
        use TestState as TS;
        self.1.retain(|el| {
            if let (Some(only), TestResult { state, .. }) = (only, el) {
                return match (only, state) {
                    (O::Fail, TS::Mismatch(..)) => true,
                    (O::Pass, TS::Correct) => true,
                    (O::Missing, TS::Missing(..)) => true,
                    _ => false,
                };
            }
            true
        });
        self
    }

    /// Print the results of running this test suite.
    pub fn print_test_suite_results(
        self: TestSuiteResult,
        opts: &cli::Opts,
        num_tests: usize,
    ) {
        use colored::*;
        let TestSuiteResult(name, results, errors) = self;

        println!("{} ({} tests)", name.bold(), num_tests);
        results
            .into_iter()
            .for_each(|info| println!("  {}", info.report_str(opts.diff)));

        if !errors.is_empty() {
            println!("  {}", "runt errors".red());
            errors
                .into_iter()
                .for_each(|info| println!("    {}", info.to_string().red()))
        }
        ()
    }
}

/// Format the output of the test into an expect string.
/// An expect string is of the form:
/// ---CODE---
/// <exit code>
/// ---STDOUT---
/// <contents of STDOUT>
/// ---STDERR---
/// <contents of STDERR>
pub fn to_expect_string(
    status: &i32,
    stdout: &String,
    stderr: &String,
) -> String {
    let mut buf = String::new();
    buf.push_str("---CODE---\n");
    buf.push_str(format!("{}", status).as_str());
    buf.push('\n');

    buf.push_str("---STDOUT---\n");
    buf.push_str(stdout.as_str());

    buf.push_str("---STDERR---\n");
    buf.push_str(stderr.as_str());

    buf.to_string()
}

/// Path of the expect file.
pub fn expect_file(path: &std::path::PathBuf) -> std::path::PathBuf {
    path.as_path().with_extension("expect")
}
