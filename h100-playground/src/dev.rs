use std::{collections::HashMap, path::Path, sync::Arc};

use anyhow::{anyhow, Result};
use bitflags::bitflags;
use rustix::{fd::OwnedFd, fs, io, mm};

pub const NVIDIA_VENDOR_ID: u16 = 0x10de;
pub const NVIDIA_HOPPER_H100: u16 = 0x2331;
pub const MEM_FILE: &str = "/dev/mem";
pub const IOMEM_FILE: &str = "/proc/iomem";

// Some important registers.
pub const NV_PMC_BOOT_0: u64 = 0x0;
pub const NV_PMC_ENABLE: u64 = 0x200;
pub const NV_PMC_DEVICE_ENABLE: u64 = 0x600;
pub const NV_CC_MODE: u64 = 0x1182cc;

/// A structure representing a base address register (BAR).
#[derive(Debug, Copy, Clone, Default)]
pub struct Bar {
    /// The address of the BAR.
    pub addr: u64,
    /// The size of the BAR.
    pub size: u64,
    /// The type of the BAR.
    pub is_64: bool,
}

bitflags! {
    #[derive(Debug)]
    pub struct CcMode : u8 {
        const CC_MODE_OFF = 0x0;
        const CC_MODE_ON = 0x1;
        const CC_MODE_DEV_TOOLS = 0x3;
    }
}

/// A structure representing the configuration of a PCI device.
///
/// Refer to the PCI Local Bus Specification, Revision 3.0 for more information.
#[derive(Debug, Clone)]
#[repr(C, align(4))]
pub struct RawConfig {
    pub vendor: u16,
    pub device: u16,
    pub status: u16,
    pub command: u16,
    pub rev_id: u8,
    pub class_code: [u8; 3],
    pub bist: u8,
    pub header_type: u8,
    pub latency_timer: u8,
    pub cache_line_size: u8,
    pub bars: [u32; 6],
    pub cardbus_cis_pointer: u32,
    pub subsystem_vendor_id: u16,
    pub subsystem_id: u16,
    pub expansion_rom_base_address: u32,
    pub capabilities_pointer: u8,
    _pad0: [u8; 3],
    _pad1: u32,
    pub max_latency: u8,
    pub min_grant: u8,
    pub interrupt_pin: u8,
    pub interrupt_line: u8,
}

#[derive(Debug)]
pub struct Config {
    pub config: RawConfig,
    pub file_fd: OwnedFd,
}

impl RawConfig {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < std::mem::size_of::<Self>() {
            return Err(anyhow!("Invalid length"));
        }

        let mut config = unsafe { std::mem::zeroed::<Self>() };
        let ptr = &mut config as *mut _ as *mut u8;

        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, std::mem::size_of::<Self>());
        }

        Ok(config)
    }
}

/// A structure representing a PCI device.
#[derive(Debug)]
pub struct PciDevice {
    /// The path
    path: String,
    /// The configuration file.
    config: Config,
    /// The capabilities of the PCI device.
    caps: HashMap<u8, u64>,
    /// The base address registers, we only need the first 6 ones.
    ///
    /// From the (incomplete) documentation provided by NVIDIA, we know that
    ///
    /// - BAR0: MMIO registers. This is the main control space of the card - all engines are controlled
    ///         through it, and it contains alternate means to access most of the other spaces.
    /// - BAR1: VRAM aperture. This is an area of prefetchable memory that maps to the card’s VRAM.
    bars: [Bar; 6],
}

/// A structure representing a GPU object.
#[derive(Debug, Clone)]
pub struct GpuObject {
    /// The PCI device.
    device: Arc<PciDevice>,
    /// The first base address register.
    bar0: Bar,
    /// base address register mappined into the memory.
    bar0_mapped: *mut u8,
}

impl PciDevice {
    /// Create a new instance of `GpuObject`.
    ///
    /// This function will open the file at the given path and read the config.
    pub fn new<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let config_path = path.as_ref().join("config");
        let file_fd = fs::open(config_path, fs::OFlags::RDONLY, fs::Mode::all())?;

        let mut buf = [0; std::mem::size_of::<RawConfig>()];
        io::read(&file_fd, &mut buf)?;

        let config = RawConfig::from_bytes(buf.as_ref())?;

        if config.device != NVIDIA_HOPPER_H100 || config.vendor != NVIDIA_VENDOR_ID {
            return Err(anyhow!(
                "Invalid device found: {}:{}",
                config.vendor,
                config.device
            ));
        }

        Ok(Self {
            path: path.as_ref().to_string_lossy().to_string(),
            config: Config { config, file_fd },
            caps: HashMap::new(),
            bars: Default::default(),
        })
    }

    /// Initialize the capabilities of the PCI device.
    pub fn init_caps(&mut self) -> Result<()> {
        if self.config.config.capabilities_pointer == 0xff {
            return Err(anyhow!("No capabilities found"));
        }

        let mut ptr = self.config.config.capabilities_pointer;

        while ptr != 0 {
            let mut data = [0u8; 4];
            fs::seek(&self.config.file_fd, fs::SeekFrom::Start(ptr as _))?;
            io::read(&self.config.file_fd, &mut data)?;

            let cap_id = data[0];
            let cap_next = data[1];

            self.caps.insert(cap_id, ptr as u64);

            ptr = cap_next;
        }

        Ok(())
    }

    /// Initialize the base address registers of the PCI device.
    pub fn init_bars(&mut self) -> Result<()> {
        let rsrc_path = format!("{}/{}", self.path, "resource");
        let raw_bars = std::fs::read_to_string(rsrc_path)?
            .split("\n")
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        let mut i = 0;
        for bar in raw_bars.iter().take(6) {
            log::info!("BAR {}: {}", i, bar);
            let bar = bar
                .split(" ")
                .map(|s| s.replace("0x", "").to_string())
                .collect::<Vec<_>>();
            let addr = u64::from_str_radix(&bar[0], 16)?;
            let end = u64::from_str_radix(&bar[1], 16)?;
            let flags = u64::from_str_radix(&bar[2], 16)?;

            // If the flag's bit 0 is set, then the BAR is not a MMIO BAR.
            if flags & 0x1 == 0 {
                // If the address is not 0, then the BAR is valid.
                if addr != 0 {
                    let size = end - addr + 1;
                    let is_64 = (flags >> 1) & 0x3 == 0x2;

                    self.bars[i] = Bar { addr, size, is_64 };

                    i += 1;
                }
            }
        }

        Ok(())
    }
}

impl GpuObject {
    /// Perform a sanity check to check if the given address is valid.
    ///
    /// This function will read the value at the given address and compare it with the value at the given
    /// address in the iomem file. If the values are not the same, then this function will return an error.
    fn sanity_check(fd: OwnedFd, addr: *const u8, target: &str) -> Result<()> {
        let boot = unsafe { std::ptr::read_volatile((addr.add(NV_PMC_BOOT_0 as _)) as *const u32) };
        if boot == 0xffffffff {
            return Err(anyhow!("sanity check of mmio failed"));
        }

        let iomem = std::fs::read_to_string(IOMEM_FILE)?
            .split("\n")
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        if let Some(line) = iomem.iter().find(|line| line.contains(target)) {
            let res = line
                .trim()
                .split(" ")
                .collect::<Vec<_>>()
                .first()
                .ok_or(anyhow!("No match found for {target}"))?
                .split("-")
                .map(|s| u64::from_str_radix(s, 16).unwrap())
                .collect::<Vec<_>>();

            let (start, end) = (res[0], res[1]);
            let size = end - start + 1;

            let mapped = unsafe {
                mm::mmap(
                    std::ptr::null_mut(),
                    size as _,
                    mm::ProtFlags::READ | mm::ProtFlags::WRITE,
                    mm::MapFlags::SHARED,
                    fd,
                    start as _,
                )? as *mut u8
            };

            let boot_val = unsafe { std::ptr::read_volatile(mapped as *const u32) };

            if boot_val != boot {
                return Err(anyhow!("sanity check of iomem failed"));
            }
        }

        Ok(())
    }

    pub fn query_cc_mode(&self) -> Result<CcMode> {
        let mode = self.read8(NV_CC_MODE)?;
        Ok(CcMode::from_bits_truncate(mode))
    }

    pub fn wait_for_boot(&self) -> Result<()> {
        Ok(())
    }

    /// Create a new instance of `GpuObject`.
    pub fn new(device: Arc<PciDevice>) -> Result<Self> {
        let fd = fs::open(MEM_FILE, fs::OFlags::RDWR, fs::Mode::all())?;
        let fd_cloned = fd.try_clone()?;
        let bar0 = device.bars[0];

        let bar0_mapped = unsafe {
            mm::mmap(
                std::ptr::null_mut(),
                bar0.size as _,
                mm::ProtFlags::READ | mm::ProtFlags::WRITE,
                mm::MapFlags::SHARED,
                fd,
                bar0.addr as _,
            )?
        } as *mut u8;

        // Do a simple sanity check to check if this register is valid.
        let boot = unsafe { std::ptr::read_volatile(bar0_mapped as *const u32) };
        if boot == 0xffffffff {
            return Err(anyhow!("sanity check of mmio failed"));
        }

        let res = Self {
            device,
            bar0,
            bar0_mapped,
        };

        GpuObject::sanity_check(fd_cloned, bar0_mapped, "nvidia")?;
        Ok(res)
    }

    pub fn get_device_handle(&self) -> Arc<PciDevice> {
        self.device.clone()
    }

    /// Read the value at the given offset.
    pub fn read(&self, offset: u64, size: u64) -> Result<Vec<u8>> {
        let mut buf = vec![0; size as _];
        let addr = self.bar0_mapped as u64 + offset;

        unsafe {
            std::ptr::copy_nonoverlapping(addr as *const u8, buf.as_mut_ptr(), size as _);
        }

        Ok(buf)
    }

    /// Write the value at the given offset.
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<()> {
        let addr = self.bar0_mapped as u64 + offset;

        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), addr as *mut u8, data.len());
        }

        Ok(())
    }

    pub fn read8(&self, offset: u64) -> Result<u8> {
        self.read(offset, 1).map(|mut buf| buf.pop().unwrap())
    }

    pub fn read16(&self, offset: u64) -> Result<u16> {
        self.read(offset, 2).map(|buf| {
            let mut data = [0; 2];
            data.copy_from_slice(&buf);
            u16::from_le_bytes(data)
        })
    }

    pub fn read32(&self, offset: u64) -> Result<u32> {
        self.read(offset, 4).map(|buf| {
            let mut data = [0; 4];
            data.copy_from_slice(&buf);
            u32::from_le_bytes(data)
        })
    }

    pub fn write8(&self, offset: u64, data: u8) -> Result<()> {
        self.write(offset, &[data])
    }

    pub fn write16(&self, offset: u64, data: u16) -> Result<()> {
        self.write(offset, &data.to_le_bytes())
    }

    pub fn write32(&self, offset: u64, data: u32) -> Result<()> {
        self.write(offset, &data.to_le_bytes())
    }
}
