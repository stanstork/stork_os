use super::sdt::SdtHeader;
use crate::cpu::io::inw;

/// Fixed ACPI Description Table (FADT)
/// The FADT is a table in the ACPI specification that provides the operating system
/// with information about the system's power management capabilities.
#[repr(C, packed)]
pub struct Fadt {
    pub header: SdtHeader,         // Common ACPI table header.
    pub firmware_ctrl: u32,        // Physical memory address of the FACS.
    pub dsdt: u32,                 // Physical memory address of the DSDT.
    pub reserved: u8,              // Reserved.
    pub preferred_pm_profile: u8,  // Preferred power management profile.
    pub sci_int: u16,              // System Control Interrupt.
    pub smi_cmd: u32,              // Port address of the SMI command port.
    pub acpi_enable: u8,           // Value to write to the SMI command port to enable ACPI.
    pub acpi_disable: u8,          // Value to write to the SMI command port to disable ACPI.
    pub s4bios_req: u8,            // Value to write to the SMI command port to enter the S4 state.
    pub pstate_cnt: u8,            // Processor performance state control.
    pub pm1a_evt_blk: u32,         // Port address of the PM1a event block.
    pub pm1b_evt_blk: u32,         // Port address of the PM1b event block.
    pub pm1a_cnt_blk: u32,         // Port address of the PM1a control block.
    pub pm1b_cnt_blk: u32,         // Port address of the PM1b control block.
    pub pm2_cnt_blk: u32,          // Port address of the PM2 control block.
    pub pm_tmr_blk: u32,           // Port address of the PM timer block.
    pub gpe0_blk: u32,             // Port address of the GPE0 block.
    pub gpe1_blk: u32,             // Port address of the GPE1 block.
    pub pm1_evt_len: u8,           // Length of the PM1 event block.
    pub pm1_cnt_len: u8,           // Length of the PM1 control block.
    pub pm2_cnt_len: u8,           // Length of the PM2 control block.
    pub pm_tmr_len: u8,            // Length of the PM timer block.
    pub gpe0_blk_len: u8,          // Length of the GPE0 block.
    pub gpe1_blk_len: u8,          // Length of the GPE1 block.
    pub gpe1_base: u8,             // Offset of the GPE1 block.
    pub cst_cnt: u8,               // Support for the _CST object.
    pub p_lvl2_lat: u16,           // Worst-case latency to enter and exit C2 state.
    pub p_lvl3_lat: u16,           // Worst-case latency to enter and exit C3 state.
    pub flush_size: u16,           // Size of the processor's flush size.
    pub flush_stride: u16,         // Stride used in the processor's flush operation.
    pub duty_offset: u8,           // Offset in the processor's duty register.
    pub duty_width: u8,            // Width of the processor's duty register.
    pub day_alarm: u8,             // Day of month to set the alarm.
    pub mon_alarm: u8,             // Month to set the alarm.
    pub century: u8,               // Century to set the alarm.
    pub iapc_boot_arch: u16,       // Boot architecture flags.
    pub reserved2: u8,             // Reserved.
    pub flags: u32,                // Miscellaneous flags.
    pub reset_reg: GenericAddress, // Reset register.
    pub reset_value: u8,           // Value to write to the reset register.
    pub arm_boot_arch: u16,        // Boot architecture flags.
    pub fadt_minor_version: u8,    // FADT minor version.
    pub x_firmware_ctrl: u64,      // Extended FACS address.
}

/// Generic Address structure used in the FADT.
#[repr(C, packed)]
pub struct GenericAddress {
    address_space_id: u8,    // Address space where the register exists.
    register_bit_width: u8,  // Size of the register in bits.
    register_bit_offset: u8, // Bit offset of the register.
    access_size: u8,         // Size of the memory access.
    address: u64,            // Address of the register.
}

impl Fadt {
    pub fn from_address(address: u64) -> &'static Fadt {
        unsafe { &*(address as *const Fadt) }
    }

    /// Checks if ACPI is enabled by reading the SCI_EN bit from the PM1a control block.
    /// Panics if ACPI is not enabled.
    pub fn ensure_acpi_enabled(&self) {
        // The SCI_EN bit is typically bit 0 in the PM1a control register. Its value is 1.
        const SCI_EN: u16 = 1;

        // The `pm1a_cnt_blk` field contains the address of the PM1a control block.
        // We cast it to a u16 to match the port I/O functions' expected input type.
        let pm1a_cnt = self.pm1a_cnt_blk as u16;

        // `inw` is a function to read a 16-bit value from the specified I/O port.
        // It reads the current value from the PM1a control block.
        let pm1a_value = inw(pm1a_cnt);

        // The function checks if the SCI_EN bit is set.
        // This is done using a bitwise AND operation between the read value and the SCI_EN mask.
        // If the SCI_EN bit is set, the result will be non-zero, indicating that ACPI is enabled.
        if (pm1a_value & SCI_EN) == 0 {
            // If ACPI is not enabled, panic with an error message.
            panic!("ACPI is not enabled!");
        }
    }
}
