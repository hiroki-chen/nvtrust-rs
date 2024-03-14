use std::{env, fs};

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use env_logger::TimestampPrecision;
use log::LevelFilter;
use nix::unistd::Uid;

pub mod bits;
pub mod cpuid;
pub mod dev;

const VERSION: &str = "535.86.06";

#[derive(Parser, Debug)]
#[command(name = "nvtrust")]
#[command(author = "Haobin Hiroki Chen. <haobchen@iu.edu>")]
#[command(version = "1.0")]
struct Cmd {
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
    // Some custom commands.
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Debug, Subcommand)]
enum SubCommand {
    #[clap(about = "Reset with OS through /sys/.../reset")]
    ResetWithOs,
    #[clap(about = "Query the current Confidential Computing (CC) mode of the GPU.")]
    QueryCcMode,
    #[clap(
        about = "Query the current Confidential Computing (CC) settings of the GPU.\nThis prints the lower level setting knobs that will take effect upon GPU reset."
    )]
    QueryCcSettings,
    #[clap(
        about = "Configure Confidentail Computing (CC) mode. The choices are off (disabled), on (enabled) or devtools (enabled in DevTools mode).\n
        The GPU needs to be reset to make the selected mode active. See --reset-after-cc-mode-switch for one way of doing it."
    )]
    SetCcMode { mode: CcModeChoice },
    #[clap(about = "Reset the GPU after switching CC mode such that it is activated immediately.")]
    ResetAfterCcModeSwitch,
    #[clap(about = "Read the physical address in the GPU's MMIO space.")]
    ReadPhys {
        #[clap(long, help = "The physical address in the GPU's MMIO space.")]
        address: u64,
        #[clap(
            long,
            help = "The output of the dumped file.",
            default_value = "dump.bin"
        )]
        output: String,
        #[clap(
            long,
            help = "The length of the data to be read.",
            default_value = "1048576"
        )]
        len: usize,
    },
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
    let args = Cmd::parse();
    init_logger(args.log);

    #[cfg(all(feature = "snp", target_arch = "x86_64"))]
    cpuid::check_sev_snp()?;

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

        log::info!("Using GPU: {}", gpu.get_name());

        match args.subcmd {
            SubCommand::ResetWithOs => {
                gpu.sysfs_reset()?;
            }
            SubCommand::QueryCcMode => {
                let cc_mode = gpu.query_cc_mode()?;
                log::info!("CC mode: {:?}", cc_mode);
            }
            SubCommand::ReadPhys {
                address,
                output,
                len,
            } => {
                log::info!("Reading {} bytes from 0x{:x} to {}", len, address, output);

                let data = gpu.read_phys(address, len)?;

                fs::write(&output, &data)?;
                log::info!("Data written to {output}, {} bytes.", data.len());
            }
            _ => log::error!("Not implemented yet."),
        }
    } else {
        log::error!("You need to be root to run this program.");
    }

    Ok(())
}
