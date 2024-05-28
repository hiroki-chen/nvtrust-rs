use bitflags::bitflags;

pub const NVIDIA_VENDOR_ID: u16 = 0x10de;
pub const NVIDIA_HOPPER_H100: u16 = 0x2331;
pub const MEM_FILE: &str = "/dev/mem";
pub const IOMEM_FILE: &str = "/proc/iomem";
pub const PCI_DEVICES: &str = "/sys/bus/pci/devices";

// Some important registers.
pub const NV_PMC_BOOT_0: u64 = 0x0;
pub const NV_PMC_ENABLE: u64 = 0x200;
pub const NV_PMC_DEVICE_ENABLE: u64 = 0x600;
/// Specify the base address of the physical address of the GPU that the host wants to read
/// through the MMIO space (see [`NV_PMC_PRAMIN_START`] - [`NV_PMC_PRAMIN_END`]).
pub const NV_HOST_MEM: u64 = 0x1700;
pub const NV_PROM_DATA: u64 = 0x300000;
pub const NV_CC_MODE: u64 = 0x1182cc;
pub const NV_PMC_PRAMIN_LEN: u64 = 1 << 20;
pub const NV_PMC_PRAMIN_START: u64 = 0x700000;
pub const NV_PMC_PRAMIN_END: u64 = NV_PMC_PRAMIN_START + NV_PMC_PRAMIN_LEN;
pub const NV_MMIO_ERROR_PREFIX: u64 = 0xbadf;
// Clocks.
pub const NV_H100_CLOCK_LOW: u64 = 0xbb0080;
pub const NV_H100_CLOCK_HIGH: u64 = 0xbb0084;

pub const PCI_CFG_SPACE_SIZE: u64 = 256;
pub const PCI_CFG_SPACE_EXP_SIZE: u64 = 4096;
pub const PCI_CAPABILITY_LIST: u64 = 0x34;
pub const PCI_CAP_ID_EXP: u64 = 0x10;
pub const PCI_CAP_ID_PM: u64 = 0x01;
pub const PCI_EXT_CAP_ID_ERR: u64 = 0x01;
pub const PCI_EXP_CAP_ID_SRIOV: u64 = 0x10;
pub const CAP_ID_MASK: u64 = 0xff;

bitflags! {
    #[derive(Debug)]
    /// NVIDIA MMIO Errors
    pub struct NvidiaMmioErrorCode: u32 {
        // ================================= //
        //            Root Error             //
        /// Nonexistent register 0xbad01XX.
        const NONEXISTENT_REG = 0xbad00100;
        /// VM fault when accessing memory.
        const VM_FAULT = 0xbad0ac00;
        // ================================= //

        /// The target refused transaction.
        const TARGET_REFUSE_TX = 0xbadf1000;
        /// No target can handle the given MMIO address.
        const NO_TARGET = 0xbadf1100;
        /// Target is explicitly disabled in PMC.ENABLE.
        const TARGET_DISABLED_PMCE = 0xbadf1200;
        /// Target is explicitly dsiabled in PRING.
        const TARGET_DISABLED_PRING = 0xbadf1300;
        /// We don't know yet.
        const OTHER_ERROR = 0xbadf5000;
    }
}

bitflags! {
  /// The Confidential Computing (CC) mode of the GPU.
  #[derive(Debug)]
  pub struct CcMode : u8 {
      /// The CC mode is off.
      const CC_MODE_OFF = 0x0;
      /// The CC mode is on.
      const CC_MODE_ON = 0x1;
      /// The CC mode is in dev tools which allows the host to do some performance tuning.
      const CC_MODE_DEV_TOOLS = 0x3;
  }
}

bitflags! {
    /// Pci Uncorrectable Errors
    pub struct PciUncorrectableErrors: u32 {
        /// Undefined error.
        const UND = 0x00000001;
        /// Data link protocol.
        const DLP = 0x00000010;
        /// Surprise down.
        const SURPDN = 0x00000020;
        /// Poisoned TLP.
        const POISON_TLP = 0x00001000;
        /// Flow control protocol.
        const FCP = 0x00002000;
        /// Completion timeout.
        const COMP_TIME = 0x0004000;

        // todo.
    }
}
