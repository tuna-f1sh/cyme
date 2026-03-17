mod common;
use std::process::Command;

/// Format an error message showing the diff between expected and actual outputs.
fn format_output_error(msg: &str, expected: &str, actual: &str) -> String {
    // Generate diff text.
    let diff_text = diff::lines(expected, actual)
        .into_iter()
        .map(|diff| match diff {
            diff::Result::Left(l) => format!("-{l}"),
            diff::Result::Both(l, _) => format!(" {l}"),
            diff::Result::Right(r) => format!("+{r}"),
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{msg}\n\nDiff:\n{diff_text}",
        msg = msg,
        diff_text = diff_text
    )
}

#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn test_lsusb_compatibility() {
    let env = common::TestEnv::new();
    let cyme_exe = env.test_exe();

    let lsusb_output = Command::new("lsusb")
        .output()
        .expect("failed to execute lsusb");
    let lsusb_stdout = String::from_utf8_lossy(&lsusb_output.stdout);

    let cyme_output = Command::new(cyme_exe)
        .arg("--lsusb")
        .output()
        .expect("failed to execute cyme");
    let cyme_stdout = String::from_utf8_lossy(&cyme_output.stdout);

    fn normalize(s: &str) -> String {
        let lines: Vec<_> = s
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        lines.join("\n")
    }

    let norm_lsusb = normalize(&lsusb_stdout);
    let norm_cyme = normalize(&cyme_stdout);

    if norm_lsusb != norm_cyme {
        panic!(
            "{}",
            format_output_error(
                "lsusb and cyme --lsusb outputs do not match.",
                &norm_lsusb,
                &norm_cyme,
            )
        );
    }
}

#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn test_lsusb_tree_compatibility() {
    let env = common::TestEnv::new();
    let cyme_exe = env.test_exe();

    let lsusb_output = Command::new("lsusb")
        .arg("-t")
        .output()
        .expect("failed to execute lsusb -t");
    let lsusb_stdout = String::from_utf8_lossy(&lsusb_output.stdout);

    let cyme_output = Command::new(cyme_exe)
        .arg("--lsusb")
        .arg("--tree")
        .output()
        .expect("failed to execute cyme --lsusb --tree");
    let cyme_stdout = String::from_utf8_lossy(&cyme_output.stdout);

    fn normalize_tree(s: &str) -> String {
        let lines: Vec<_> = s
            .lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .collect();
        lines.join("\n")
    }

    let norm_lsusb = normalize_tree(&lsusb_stdout);
    let norm_cyme = normalize_tree(&cyme_stdout);

    if norm_lsusb != norm_cyme {
        panic!(
            "{}",
            format_output_error(
                "lsusb -t and cyme --lsusb --tree outputs do not match.",
                &norm_lsusb,
                &norm_cyme,
            )
        );
    }
}

#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn test_lsusb_tree_verbose_compatibility() {
    let env = common::TestEnv::new();
    let cyme_exe = env.test_exe();

    let lsusb_output = Command::new("lsusb")
        .arg("-t")
        .arg("-v")
        .output()
        .expect("failed to execute lsusb -t -v");
    let lsusb_stdout = String::from_utf8_lossy(&lsusb_output.stdout);

    let cyme_output = Command::new(cyme_exe)
        .arg("--lsusb")
        .arg("--tree")
        .arg("--verbose")
        .output()
        .expect("failed to execute cyme --lsusb --tree --verbose");
    let cyme_stdout = String::from_utf8_lossy(&cyme_output.stdout);

    fn normalize_tree(s: &str) -> String {
        let lines: Vec<_> = s
            .lines()
            .map(|l| l.trim_end())
            .filter(|l| !l.is_empty())
            .collect();
        lines.join("\n")
    }

    let norm_lsusb = normalize_tree(&lsusb_stdout);
    let norm_cyme = normalize_tree(&cyme_stdout);

    if norm_lsusb != norm_cyme {
        panic!(
            "{}",
            format_output_error(
                "lsusb -t -v and cyme --lsusb --tree --verbose outputs do not match.",
                &norm_lsusb,
                &norm_cyme,
            )
        );
    }
}
