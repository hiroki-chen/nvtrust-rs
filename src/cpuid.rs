use anyhow::{anyhow, Result};
use log::info;
use x86::cpuid;

#[cfg(all(feature = "snp", target_arch = "x86_64"))]
pub fn check_sev_snp() -> Result<()> {
    let cpuid = cpuid::CpuId::new();
    let svm = cpuid.get_svm_info().ok_or(anyhow!(
        "no svm information detected: is this a SNP-supported CPU?"
    ))?;

    if svm.has_nested_paging() {
        info!("detected AMD-SEV-SNP feature!");
        info!("ASID: 0x{:x}", svm.supported_asids());

        /*
         * Check for the SME/SEV feature:
         *   CPUID Fn8000_001F[EAX]
         *   - Bit 0 - Secure Memory Encryption support
         *   - Bit 1 - Secure Encrypted Virtualization support
         *   CPUID Fn8000_001F[EBX]
         *   - Bits 5:0 - Pagetable bit position used to indicate encryption
         */
        let raw_info = cpuid::cpuid!(0x8000_001f, 0x0);
        let sme_me_mask = 1 << (raw_info.ebx & 0x3f);

        info!("SME/ME Mask: {:#064b}", sme_me_mask);

        Ok(())
    } else {
        Err(anyhow!("nested paging is not supported"))
    }
}
