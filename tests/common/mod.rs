//! Runs tests using actual binary, apapted from 'fd' method: https://github.com/sharkdp/fd/blob/master/tests/testenv/mod.rs
#![allow(dead_code)]
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::process;
// #[cfg(windows)]
// use std::os::windows;

// if changing content of USBDeviceX structs, update the tests data with `--from-json TEST_DUMP --json TEST_ARGS > file.json`
/// Dump from the `system_profiler` command on macOS
pub const SYSTEM_PROFILER_DUMP_PATH: &str = "./tests/data/system_profiler_dump.json";
/// Dump using macOS system_profiler so no [`USBDeviceExtra`]
pub const CYME_SP_TREE_DUMP: &str = "./tests/data/cyme_sp_macos_tree.json";
/// Dump using macOS system_profiler and libusb merge so with [`USBDeviceExtra`]
pub const CYME_LIBUSB_MERGE_MACOS_TREE_DUMP: &str =
    "./tests/data/cyme_libusb_merge_macos_tree.json";
/// Dump using macOS force libusb merge so with [`USBDeviceExtra`] but not Apple internal buses
pub const CYME_LIBUSB_MACOS_TREE_DUMP: &str = "./tests/data/cyme_libusb_macos_tree.json";
/// Dump using Linux with libusb so with [`USBDeviceExtra`]
pub const CYME_LIBUSB_LINUX_TREE_DUMP: &str = "./tests/data/cyme_libusb_linux_tree.json";
/// Output of lsusb --tree
pub const LSUSB_TREE_OUTPUT: &str = "./tests/data/lsusb_tree.txt";
/// Output of lsusb --tree -vvv
pub const LSUSB_TREE_OUTPUT_VERBOSE: &str = "./tests/data/lsusb_tree_verbose.txt";
/// Output of lsusb
pub const LSUSB_OUTPUT: &str = "./tests/data/lsusb_list.txt";
/// Output of lsusb --verbose
pub const LSUSB_OUTPUT_VERBOSE: &str = "./tests/data/lsusb_verbose.txt";

pub fn read_dump(file_name: &str) -> BufReader<File> {
    let f = File::open(file_name).expect("Unable to open json dump file");
    BufReader::new(f)
}

pub fn read_dump_to_string(file_name: &str) -> String {
    let mut ret = String::new();
    let mut br = read_dump(file_name);
    br.read_to_string(&mut ret)
        .unwrap_or_else(|_| panic!("Failed to read {}", file_name));
    ret
}

pub fn sp_data_from_system_profiler() -> cyme::system_profiler::SPUSBDataType {
    let mut br = read_dump(SYSTEM_PROFILER_DUMP_PATH);
    let mut data = String::new();
    br.read_to_string(&mut data).expect("Unable to read string");

    serde_json::from_str::<cyme::system_profiler::SPUSBDataType>(&data).unwrap()
}

pub fn sp_data_from_libusb_linux() -> cyme::system_profiler::SPUSBDataType {
    let mut br = read_dump(CYME_LIBUSB_LINUX_TREE_DUMP);
    let mut data = String::new();
    br.read_to_string(&mut data).expect("Unable to read string");

    serde_json::from_str::<cyme::system_profiler::SPUSBDataType>(&data).unwrap()
}

/// Environment for the integration tests.
pub struct TestEnv {
    /// Path to the *cyme* executable.
    cyme_exe: PathBuf,
    /// Normalize each line by sorting the whitespace-separated words
    normalize_line: bool,
    /// Strip whitespace at start
    strip_start: bool,
}

/// Find the *cyme* executable.
fn find_cyme_exe() -> PathBuf {
    // Tests exe is in target/debug/deps, the *cyme* exe is in target/debug
    let root = env::current_exe()
        .expect("tests executable")
        .parent()
        .expect("tests executable directory")
        .parent()
        .expect("cyme executable directory")
        .to_path_buf();

    let exe_name = if cfg!(windows) { "cyme.exe" } else { "cyme" };

    root.join(exe_name)
}

/// Format an error message for when *cyme* did not exit successfully.
fn format_exit_error(args: &[&str], output: &process::Output) -> String {
    format!(
        "`cyme {}` did not exit successfully.\nstdout:\n---\n{}---\nstderr:\n---\n{}---",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Format an error message for when the output of *cyme* did not match the expected output.
fn format_output_error(args: &[&str], expected: &str, actual: &str) -> String {
    // Generate diff text.
    let diff_text = diff::lines(expected, actual)
        .into_iter()
        .map(|diff| match diff {
            diff::Result::Left(l) => format!("-{}", l),
            diff::Result::Both(l, _) => format!(" {}", l),
            diff::Result::Right(r) => format!("+{}", r),
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        concat!(
            "`cyme {}` did not produce the expected output.\n",
            "Showing diff between expected and actual:\n{}\n"
        ),
        args.join(" "),
        diff_text
    )
}

/// Normalize the output for comparison.
fn normalize_output(s: &str, trim_start: bool, normalize_line: bool) -> String {
    // Split into lines and normalize separators.
    let mut lines = s
        .replace('\0', "NULL\n")
        .lines()
        .map(|line| {
            let line = if trim_start { line.trim_start() } else { line };
            if normalize_line {
                let mut words: Vec<_> = line.split_whitespace().collect();
                words.sort_unstable();
                return words.join(" ");
            }
            line.to_string()
        })
        .collect::<Vec<_>>();

    lines.sort();
    lines.join("\n")
}

/// Trim whitespace from the beginning of each line.
fn trim_lines(s: &str) -> String {
    s.lines()
        .map(|line| line.trim_start())
        .fold(String::new(), |mut str, line| {
            str.push_str(line);
            str.push('\n');
            str
        })
}

impl TestEnv {
    pub fn new() -> TestEnv {
        let cyme_exe = find_cyme_exe();

        TestEnv {
            cyme_exe,
            normalize_line: false,
            strip_start: false,
        }
    }

    pub fn normalize_line(self, normalize: bool, strip_start: bool) -> TestEnv {
        TestEnv {
            cyme_exe: self.cyme_exe,
            normalize_line: normalize,
            strip_start,
        }
    }

    /// Get the path of the cyme executable.
    #[cfg_attr(windows, allow(unused))]
    pub fn test_exe(&self) -> &PathBuf {
        &self.cyme_exe
    }

    /// Assert that calling *cyme* in the specified path under the root working directory,
    /// and with the specified arguments produces the expected output.
    pub fn assert_success_and_get_output(
        &self,
        dump_file: Option<&str>,
        args: &[&str],
    ) -> process::Output {
        // Setup *cyme* command.
        let mut cmd = process::Command::new(&self.cyme_exe);
        if let Some(dump) = dump_file {
            cmd.arg("--from-json").arg(dump).args(args);
        } else {
            cmd.arg("--json").args(args);
        }

        // Run *cyme*.
        let output = cmd.output().expect("cyme output");

        // Check for exit status.
        if !output.status.success() {
            panic!("{}", format_exit_error(args, &output));
        }

        output
    }

    pub fn assert_success_and_get_normalized_output(
        &self,
        dump_file: Option<&str>,
        args: &[&str],
    ) -> String {
        let output = self.assert_success_and_get_output(dump_file, args);
        normalize_output(
            &String::from_utf8_lossy(&output.stdout),
            self.strip_start,
            self.normalize_line,
        )
    }

    /// Assert that calling *cyme* with the specified arguments produces the expected output.
    pub fn assert_output(
        &self,
        dump_file: Option<&str>,
        args: &[&str],
        expected: &str,
        contains: bool,
    ) {
        // Don't touch if doing contains
        let (expected, actual) = if contains {
            let output = self.assert_success_and_get_output(dump_file, args);
            (
                expected.to_string(),
                String::from_utf8_lossy(&output.stdout).to_string(),
            )
        // Normalize both expected and actual output.
        } else {
            (
                normalize_output(expected, self.strip_start, self.normalize_line),
                self.assert_success_and_get_normalized_output(dump_file, args),
            )
        };

        // Compare actual output to expected output.
        if contains {
            if !actual.contains(&expected) {
                panic!("{}", format_output_error(args, &expected, &actual));
            }
        } else if expected != actual {
            panic!("{}", format_output_error(args, &expected, &actual));
        }
    }

    pub fn assert_output_json(&self, dump_file: Option<&str>, args: &[&str], expected: &str) {
        // Normalize both expected and actual output.
        let output = self.assert_success_and_get_output(dump_file, args);
        let actual = String::from_utf8_lossy(&output.stdout).to_string();

        // Compare actual output to expected output.
        assert_json_diff::assert_json_include!(actual: json!(actual), expected: json!(expected));
    }

    /// Parses output back to SPUSBDataType and checks device with `port_path` exists in it
    pub fn assert_output_contains_port_path(
        &self,
        dump_file: Option<&str>,
        args: &[&str],
        port_path: &str,
    ) {
        // Normalize both expected and actual output.
        let output = self.assert_success_and_get_output(dump_file, args);
        let actual = String::from_utf8_lossy(&output.stdout).to_string();
        let spdata_out =
            serde_json::from_str::<cyme::system_profiler::SPUSBDataType>(&actual).unwrap();

        assert!(spdata_out.get_node(port_path).is_some());
    }

    /// Similar to assert_output, but able to handle non-utf8 output
    #[cfg(all(unix, not(target_os = "macos")))]
    pub fn assert_output_raw(&self, dump_file: Option<&str>, args: &[&str], expected: &[u8]) {
        let output = self.assert_success_and_get_output(dump_file, args);

        assert_eq!(expected, &output.stdout[..]);
    }

    /// Assert that calling *cyme* with the specified arguments produces the expected error,
    /// and does not succeed.
    pub fn assert_failure_with_error(
        &self,
        dump_file: Option<&str>,
        args: &[&str],
        expected: &str,
    ) {
        let status = self.assert_error(dump_file, args, Some(expected));
        if status.success() {
            panic!("error '{}' did not occur.", expected);
        }
    }

    /// Assert that calling *cyme* with the specified arguments does not succeed.
    pub fn assert_failure(&self, dump_file: Option<&str>, args: &[&str]) {
        let status = self.assert_error(dump_file, args, None);
        if status.success() {
            panic!("Failure did not occur as expected.");
        }
    }

    fn assert_error(
        &self,
        dump_file: Option<&str>,
        args: &[&str],
        expected: Option<&str>,
    ) -> process::ExitStatus {
        // Setup *cyme* command.
        let mut cmd = process::Command::new(&self.cyme_exe);
        if let Some(dump) = dump_file {
            cmd.arg("--from-json").arg(dump).args(args);
        } else {
            cmd.arg("--json").args(args);
        }

        // Run *cyme*.
        let output = cmd.output().expect("cyme output");

        if let Some(expected) = expected {
            // Normalize both expected and actual output.
            let expected_error = trim_lines(expected);
            let actual_err = trim_lines(&String::from_utf8_lossy(&output.stderr));

            // Compare actual output to expected output.
            if !actual_err.trim_start().starts_with(&expected_error) {
                panic!(
                    "{}",
                    format_output_error(args, &expected_error, &actual_err)
                );
            }
        }

        output.status
    }
}
