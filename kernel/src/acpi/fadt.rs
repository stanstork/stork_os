use core::fmt;

use crate::{
    cpu::io::{inw, outb, sleep_for, Port, PortIO},
    println,
};

use super::sdt::SdtHeader;

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
    pub x_firmware_ctrl: u64,
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

    pub fn enable_acpi(&self) {
        const SCI_EN: u16 = 1;

        let pm1a_cnt = self.pm1a_cnt_blk as u16;
        let pm1b_cnt = self.pm1b_cnt_blk as u16;

        // Check if the SCI_EN bit is set in the PM1 control register.
        if (inw(pm1a_cnt) & SCI_EN) == 0 {
            if self.smi_cmd != 0 && self.acpi_enable != 0 {
                // Write the ACPI enable value to the SMI command port.
                outb(self.smi_cmd as u16, self.acpi_enable);

                // Wait for the ACPI enable to be processed.
                let mut enabled = false;
                let mut i = 0;
                while i < 300 {
                    if (inw(pm1a_cnt) & SCI_EN) != 0 {
                        enabled = true;
                        break;
                    }
                    sleep_for(10);
                    i += 1;
                }

                if pm1b_cnt != 0 {
                    let mut i = 0;
                    while i < 300 {
                        if (inw(pm1b_cnt) & SCI_EN) != 0 {
                            enabled = true;
                            break;
                        }
                        sleep_for(10);
                        i += 1;
                    }
                }

                if !enabled {
                    println!("ACPI enable failed");
                } else {
                    println!("ACPI enabled");
                }
            } else {
                println!("No SMI command port or ACPI enable value");
            }
        } else {
            println!("ACPI already enabled");
        }
    }
}

impl fmt::Debug for Fadt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Fadt {{ ")?;
        write!(f, "header: {:?}, ", self.header)?;
        let firmware_ctrl = self.firmware_ctrl;
        write!(f, "firmware_ctrl: {:#X}, ", firmware_ctrl)?;
        let dsdt = self.dsdt;
        write!(f, "dsdt: {:#X}, ", dsdt)?;
        write!(f, "reserved: {:#X}, ", self.reserved)?;
        write!(
            f,
            "preferred_pm_profile: {:#X}, ",
            self.preferred_pm_profile
        )?;
        let sci_int = self.sci_int;
        write!(f, "sci_int: {:#X}, ", sci_int)?;
        let smi_cmd = self.smi_cmd;
        write!(f, "smi_cmd: {:#X}, ", smi_cmd)?;
        write!(f, "acpi_enable: {:#X}, ", self.acpi_enable)?;
        write!(f, "acpi_disable: {:#X}, ", self.acpi_disable)?;
        write!(f, "s4bios_req: {:#X}, ", self.s4bios_req)?;
        write!(f, "pstate_cnt: {:#X}, ", self.pstate_cnt)?;
        let pm1a_evt_blk = self.pm1a_evt_blk;
        write!(f, "pm1a_evt_blk: {:#X}, ", pm1a_evt_blk)?;
        let pm1b_evt_blk = self.pm1b_evt_blk;
        write!(f, "pm1b_evt_blk: {:#X}, ", pm1b_evt_blk)?;
        let pm1a_cnt_blk = self.pm1a_cnt_blk;
        write!(f, "pm1a_cnt_blk: {:#X}, ", pm1a_cnt_blk)?;
        let pm1b_cnt_blk = self.pm1b_cnt_blk;
        write!(f, "pm1b_cnt_blk: {:#X}, ", pm1b_cnt_blk)?;
        let pm2_cnt_blk = self.pm2_cnt_blk;
        write!(f, "pm2_cnt_blk: {:#X}, ", pm2_cnt_blk)?;
        let pm_tmr_blk = self.pm_tmr_blk;
        write!(f, "pm_tmr_blk: {:#X}, ", pm_tmr_blk)?;
        let gpe0_blk = self.gpe0_blk;
        write!(f, "gpe0_blk: {:#X}, ", gpe0_blk)?;
        let gpe1_blk = self.gpe1_blk;
        write!(f, "gpe1_blk: {:#X}, ", gpe1_blk)?;
        write!(f, "pm1_evt_len: {:#X}, ", self.pm1_evt_len)?;
        write!(f, "pm1_cnt_len: {:#X}, ", self.pm1_cnt_len)?;
        write!(f, "pm2_cnt_len: {:#X}, ", self.pm2_cnt_len)?;
        write!(f, "pm_tmr_len: {:#X}, ", self.pm_tmr_len)?;
        write!(f, "gpe0_blk_len: {:#X}, ", self.gpe0_blk_len)?;
        write!(f, "gpe1_blk_len: {:#X}, ", self.gpe1_blk_len)?;
        write!(f, "gpe1_base: {:#X}, ", self.gpe1_base)?;
        write!(f, "cst_cnt: {:#X}, ", self.cst_cnt)?;
        let p_lvl2_lat = self.p_lvl2_lat;
        write!(f, "p_lvl2_lat: {:#X}, ", p_lvl2_lat)?;
        let p_lvl3_lat = self.p_lvl3_lat;
        write!(f, "p_lvl3_lat: {:#X}, ", p_lvl3_lat)?;
        let flush_size = self.flush_size;
        write!(f, "flush_size: {:#X}, ", flush_size)?;
        let flush_stride = self.flush_stride;
        write!(f, "flush_stride: {:#X}, ", flush_stride)?;
        write!(f, "duty_offset: {:#X}, ", self.duty_offset)?;
        write!(f, "duty_width: {:#X}, ", self.duty_width)?;
        write!(f, "day_alarm: {:#X}, ", self.day_alarm)?;
        write!(f, "mon_alarm: {:#X}, ", self.mon_alarm)?;
        write!(f, "century: {:#X}, ", self.century)?;
        let iapc_boot_arch = self.iapc_boot_arch;
        write!(f, "iapc_boot_arch: {:#X}, ", iapc_boot_arch)?;
        write!(f, "reserved2: {:#X}, ", self.reserved2)?;
        let flags = self.flags;
        write!(f, "flags: {:#X}, ", flags)?;
        write!(f, "reset_reg: {:?}, ", self.reset_reg)?;
        write!(f, "reset_value: {:#X}, ", self.reset_value)?;
        let arm_boot_arch = self.arm_boot_arch;
        write!(f, "arm_boot_arch: {:#X}, ", arm_boot_arch)?;
        write!(f, "fadt_minor_version: {:#X}, ", self.fadt_minor_version)?;
        let x_firmware_ctrl = self.x_firmware_ctrl;
        write!(f, "x_firmware_ctrl: {:#X} ", x_firmware_ctrl)?;
        write!(f, "}}")
    }
}

impl fmt::Debug for GenericAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenericAddress {{ ")?;
        write!(f, "address_space_id: {:#X}, ", self.address_space_id)?;
        write!(f, "register_bit_width: {:#X}, ", self.register_bit_width)?;
        write!(f, "register_bit_offset: {:#X}, ", self.register_bit_offset)?;
        write!(f, "access_size: {:#X}, ", self.access_size)?;

        let address = self.address;
        write!(f, "address: {:#X} ", address)?;

        write!(f, "}}")
    }
}
