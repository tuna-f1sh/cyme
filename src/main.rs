use clap::Parser;
use std::env;

mod system_profiler;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Attempt to maintain compatibility with lsusb output
    #[arg(short, long, default_value_t = false)]
    lsusb: bool,

    /// Disable coloured output, can also use NO_COLOR environment variable
    #[arg(short, long, default_value_t = false)]
    no_colour: bool,

    /// Classic dump the physical USB device hierarchy as a tree
    #[arg(short = 't', long, default_value_t = false)]
    lsusb_tree: bool,

    /// Modern dump the physical USB device hierarchy as a tree
    #[arg(short = 'T', long, default_value_t = true)]
    tree: bool,

    /// Show only devices with the specified vendor and product ID numbers (in hexadecimal) in format VID:[PID]
    #[arg(short, long)]
    device: Option<String>,

    /// Show only devices with specified device and/or bus numbers (in decimal) in format [[bus]:][devnum]
    #[arg(short, long)]
    show: Option<String>,

    /// Increase verbosity (show descriptors)
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    let sp_usb = system_profiler::get_spusb().unwrap();

    // just set the env for this process
    if args.no_colour {
        env::set_var("NO_COLOR", "1");
    }

    if args.lsusb {
        if args.lsusb_tree {
            print!("{:+}", sp_usb);
        } else {
            print!("{:}", sp_usb);
        }
    } else {
        if args.tree {
            print!("{:+#}", sp_usb);
        } else {
            print!("{:#}", sp_usb);
        }
    }
}
