use std::env;

use anyhow::Result;
use nix::unistd::Uid;

pub mod dev;

use crate::dev::{GpuObject, NV_PMC_BOOT_0};

static MEM_FILE: &str = "/sys/bus/pci/devices/0000:41:00.0";

fn init_logger() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::init();
}

fn main() -> Result<()> {
    init_logger();

    if Uid::effective().is_root() {
        let mut dev = dev::PciDevice::new(MEM_FILE)?;
        dev.init_caps()?;
        dev.init_bars()?;

        let mut gpu = GpuObject::new(dev.into())?;
        log::info!("{gpu:x?}");
        let id = gpu.read32(0x1182cc)?;

        log::info!("ID: {:#x}", id);
        log::info!("CC-mode: {:?}", gpu.query_cc_mode()?);
    } else {
        log::error!("You need to be root to run this program.+");
    }

    Ok(())
}
