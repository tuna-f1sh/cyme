extern crate clap;
extern crate version_check;

use std::fs;
use std::process::exit;
use std::path::PathBuf;
use clap::CommandFactory;
use clap_complete::generate_to;
use clap_complete::shells::*;

include!("src/app.rs");

fn main() -> std::io::Result<()> {
    // rustc version too small or can't figure it out
    if version_check::is_min_version("1.64.0") != Some(true) {
        eprintln!("'cyme' requires rustc >= 1.64.0");
        exit(1);
    }

    let outdir = std::env::var_os("BUILD_SCRIPT_DIR")
        .or_else(|| std::env::var_os("OUT_DIR"))
        .unwrap_or_else(|| exit(0));

    fs::create_dir_all(&outdir).unwrap();

    let mut app = <Args as CommandFactory>::command();

    let bin_name = "cyme";
    generate_to(Bash, &mut app, bin_name, &outdir).expect("Failed to generate Bash completions");
    generate_to(Fish, &mut app, bin_name, &outdir).expect("Failed to generate Fish completions");
    generate_to(Zsh, &mut app, bin_name, &outdir).expect("Failed to generate Zsh completions");
    generate_to(PowerShell, &mut app, bin_name, &outdir)
        .expect("Failed to generate PowerShell completions");

    let man = clap_mangen::Man::new(app);
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;

    std::fs::write(PathBuf::from(outdir).join("mybin.1"), buffer)?;

    Ok(())
}
