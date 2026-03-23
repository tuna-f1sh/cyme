#[cfg(target_os = "linux")]
mod common;
#[cfg(target_os = "linux")]
use std::process::Command;

/// Format an error message showing the diff between expected and actual outputs.
#[allow(dead_code)]
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

/// Test for lsusb -d 1d6b:0001. Below tests use root_hub device which _should_ always be present and has a fixed Linux VID:PID.
#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn test_lsusb_d_arg() {
    let env = common::TestEnv::new();
    let cyme_exe = env.test_exe();

    let lsusb_output = Command::new("lsusb")
        .arg("-d")
        .arg("1d6b:0001")
        .output()
        .expect("failed to execute lsusb -d 1d6b:0001");
    let lsusb_stdout = String::from_utf8_lossy(&lsusb_output.stdout);

    let cyme_output = Command::new(cyme_exe)
        .arg("--lsusb")
        .arg("-d")
        .arg("1d6b:0001")
        .output()
        .expect("failed to execute cyme --lsusb -d 1d6b:0001");
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
                "lsusb -d and cyme --lsusb --device outputs do not match.",
                &norm_lsusb,
                &norm_cyme,
            )
        );
    }
}

// /// Test for lsusb --device /dev/bus/usb/001/001.
// #[test]
// #[ignore]
// #[cfg(target_os = "linux")]
// fn test_lsusb_device_arg() {
//     let env = common::TestEnv::new();
//     let cyme_exe = env.test_exe();
//
//     let lsusb_output = Command::new("lsusb")
//         .arg("-D")
//         .arg("/dev/bus/usb/001/001")
//         .output()
//         .expect("failed to execute lsusb --device /dev/bus/usb/001/001");
//     let lsusb_stdout = String::from_utf8_lossy(&lsusb_output.stdout);
//
//     let cyme_output = Command::new(cyme_exe)
//         .arg("--lsusb")
//         .arg("-D")
//         .arg("/dev/bus/usb/001/001")
//         .output()
//         .expect("failed to execute cyme --lsusb -D /dev/bus/usb/001/001");
//     let cyme_stdout = String::from_utf8_lossy(&cyme_output.stdout);
//
//     fn normalize(s: &str) -> String {
//         let lines: Vec<_> = s
//             .lines()
//             .map(|l| l.trim())
//             .filter(|l| !l.is_empty())
//             .collect();
//         lines.join("\n")
//     }
//
//     let norm_lsusb = normalize(&lsusb_stdout);
//     let norm_cyme = normalize(&cyme_stdout);
//
//     if norm_lsusb != norm_cyme {
//         panic!(
//             "{}",
//             format_output_error(
//                 "lsusb --device and cyme --lsusb --device outputs do not match.",
//                 &norm_lsusb,
//                 &norm_cyme,
//             )
//         );
//     }
// }

/// Test for lsusb -s 001:
#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn test_lsusb_s_arg() {
    let env = common::TestEnv::new();
    let cyme_exe = env.test_exe();

    let lsusb_output = Command::new("lsusb")
        .arg("-s")
        .arg("001:")
        .output()
        .expect("failed to execute lsusb -s 001:");
    let lsusb_stdout = String::from_utf8_lossy(&lsusb_output.stdout);

    let cyme_output = Command::new(cyme_exe)
        .arg("--lsusb")
        .arg("-s")
        .arg("001:")
        .output()
        .expect("failed to execute cyme --lsusb -s 001:");
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
                "lsusb -s and cyme --lsusb --bus outputs do not match.",
                &norm_lsusb,
                &norm_cyme,
            )
        );
    }
}

/// Test for lsusb -s 001:001
#[test]
#[ignore]
#[cfg(target_os = "linux")]
fn test_lsusb_s_arg_full() {
    let env = common::TestEnv::new();
    let cyme_exe = env.test_exe();

    let lsusb_output = Command::new("lsusb")
        .arg("-s")
        .arg("001:001")
        .output()
        .expect("failed to execute lsusb -s 001:001");
    let lsusb_stdout = String::from_utf8_lossy(&lsusb_output.stdout);

    let cyme_output = Command::new(cyme_exe)
        .arg("--lsusb")
        .arg("-s")
        .arg("001:001")
        .output()
        .expect("failed to execute cyme --lsusb -s 001:001");
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
                "lsusb -s and cyme --lsusb --device outputs do not match.",
                &norm_lsusb,
                &norm_cyme,
            )
        );
    }
}
