use std::env;

use anyhow::Result;
use clap::{Parser, ValueEnum};
use env_logger::TimestampPrecision;
use log::LevelFilter;
use nix::unistd::Uid;

pub mod bits;
pub mod dev;

const VERSION: &str = "535.86.06";

#[derive(Parser, Debug)]
#[command(name = "nvtrust")]
#[command(author = "Haobin Hiroki Chen. <haobchen@iu.edu>")]
#[command(version = "1.0")]
struct Args {
    #[clap(long, help = "Select the index of the GPU.", default_value = "-1")]
    gpu: Option<i64>,
    #[clap(
        long,
        help = "Select a single GPU by providing a substring of the BDF, e.g. '01:00'."
    )]
    gpu_bdf: Option<String>,
    #[clap(
        long,
        help = "Select a single GPU by providing a substring of the GPU name, e.g. 'T4'. If multiple GPUs match, the first one will be used."
    )]
    gpu_name: Option<String>,
    #[clap(
        long,
        help = "Do not use any of the GPUs; commands requiring one will not work.",
        default_value = "false"
    )]
    no_gpu: bool,
    #[clap(long, default_value = "info")]
    log: LevelFilter,
    #[clap(
        long,
        help = "Reset with OS through /sys/.../reset",
        default_value = "false"
    )]
    reset_with_os: bool,
    #[clap(
        long,
        help = "Query the current Confidential Computing (CC) mode of the GPU.",
        default_value = "false"
    )]
    query_cc_mode: bool,
    #[clap(
        long,
        help = "Query the current Confidential Computing (CC) settings of the GPU.\nThis prints the lower level setting knobs that will take effect upon GPU reset.",
        default_value = "false"
    )]
    query_cc_settings: bool,
    #[clap(
        long,
        help = "Configure Confidentail Computing (CC) mode. The choices are off (disabled), on (enabled) or devtools (enabled in DevTools mode).\n
        The GPU needs to be reset to make the selected mode active. See --reset-after-cc-mode-switch for one way of doing it."
    )]
    set_cc_mode: Option<CcModeChoice>,
    #[clap(
        long,
        help = "Reset the GPU after switching CC mode such that it is activated immediately.",
        default_value = "false"
    )]
    reset_after_cc_mode_switch: bool,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CcModeChoice {
    /// Disable CC mode.
    Off,
    /// Enable CC mode.
    On,
    /// Enable CC mode in DevTools mode.
    DevTools,
}

fn init_logger(level: LevelFilter) {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::builder()
        .filter_level(level)
        .format_module_path(false)
        .format_target(false)
        .format_timestamp(Some(TimestampPrecision::Seconds))
        .init();
}

fn main() -> Result<()> {
    let args = Args::parse();
    init_logger(args.log);
    log::info!("NVIDIA GPU Tools version {VERSION}");

    if Uid::effective().is_root() {
        let gpu = {
            if let Some(bdf) = args.gpu_bdf {
                let gpus = dev::find_gpus_by_bdf(&bdf)?;

                if gpus.is_empty() {
                    log::error!("Matching for {bdf} found nothing");

                    return Ok(());
                } else if gpus.len() > 1 {
                    log::warn!(
                        "Matching for {bdf} found multiple GPUs: {:?}. Use the first one.",
                        gpus,
                    );

                    gpus[0].clone()
                } else {
                    gpus[0].clone()
                }
            } else {
                log::error!("No GPU specified, select GPU with --gpu, --gpu-bdf, or --gpu-name.");
                return Ok(());
            }
        };

        if args.reset_with_os {
            gpu.sysfs_reset()?;
        }

        if args.query_cc_mode {
            let cc_mode = gpu.query_cc_mode()?;
            log::info!("CC settings: {:?}", cc_mode);
        }
    } else {
        log::error!("You need to be root to run this program.+");
    }

    Ok(())
}
