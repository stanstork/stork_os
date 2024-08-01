use crate::{
    cpu::io::{Port, PortIO},
    interrupts::isr::{IDT, KEYBOARD_IRQ, TIMER_IRQ},
    memory::{self},
};

// LAPIC Local Vector Table (LVT) Registers
pub const LVT_TIMER: u32 = 0x320; // Local Vector Timer Register
pub const LVT_THERMAL: u32 = 0x330; // Local Vector Thermal Sensor Register
pub const LVT_PERFMON: u32 = 0x340; // Local Vector Performance Monitoring Register
pub const LVT_LINT0: u32 = 0x350; // Local Vector Local Interrupt 0 Register
pub const LVT_LINT1: u32 = 0x360; // Local Vector Local Interrupt 1 Register
pub const LVT_ERROR: u32 = 0x370; // Local Vector Error Register
pub const LVT_EOI: u32 = 0xB0; // Local Vector EOI Register
pub const LVT_SPV: u32 = 0xF0; // Local Vector Spurious Interrupt Vector Register
pub const LVT_TPR: u32 = 0x80; // Local Vector Task Priority Register

// LAPIC Timer Configuration Registers
pub const TIMER_DIVIDE_CONFIG_REG: u32 = 0x3E0; // Timer Divide Configuration Register
pub const TIMER_INITIAL_COUNT_REG: u32 = 0x380; // Timer Initial Count Register

// LAPIC LVT Flags
pub const APIC_LVT_MASKED: u32 = 0x10000; // Masked flag for LVT registers
pub const APIC_TIMER_PERIODIC_MODE: u32 = 1 << 17; // Timer periodic mode flag
pub const APIC_SPURIOUS_INTERRUPT_VECTOR_ENABLE: u32 = 0x100; // Spurious interrupt vector enable flag

// PIC and IMCR Ports
pub const PIC_IMCR_SELECT: u16 = 0x22;
pub const PIC_IMCR_DATA: u16 = 0x23;
pub const IMCR_SELECT_REGISTER: u8 = 0x70;
pub const IMCR_DISABLE_PIC: u8 = 0x01;

// LAPIC Timer Configuration
pub const LAPIC_TIMER_VECTOR: u32 = 32;
pub const LAPIC_TIMER_INITIAL_COUNT: u32 = 400_000; // Calculated initial count for 250 Hz (assume)
pub const LAPIC_TIMER_DIVIDE_CONFIG: u32 = 0x1; // Divide by 1

/// Represents the Local APIC (LAPIC) structure.
pub struct Lapic {
    pub local_apic_address: *mut u32,
}

impl Lapic {
    /// Creates a new LAPIC instance with the specified local APIC address.
    pub const fn new(local_apic_address: u32) -> Lapic {
        Lapic {
            local_apic_address: local_apic_address as *mut u32,
        }
    }

    /// Enables APIC mode and configures necessary settings.
    pub fn enable(&self) {
        // Disable the PIC by masking its interrupts
        unsafe {
            IDT.disable_pic_interrupt(KEYBOARD_IRQ);
            IDT.disable_pic_interrupt(TIMER_IRQ);
        }

        // Enable the APIC by setting the APIC enable bit in the APIC base address register
        Port::new(PIC_IMCR_SELECT).write_port(IMCR_SELECT_REGISTER); // Select IMCR at 0x70
        Port::new(PIC_IMCR_DATA).write_port(IMCR_DISABLE_PIC); // Set IMCR to disconnect PIC from LINT0

        // Map the LAPIC memory address
        memory::map_io(self.local_apic_address as u64);

        // Initialize the LAPIC
        self.init();
    }

    /// Sends an End-of-Interrupt (EOI) signal to the LAPIC.
    pub fn eoi(&self) {
        self.write_register(LVT_EOI, 0);
    }

    /// Initializes the LAPIC by setting up the vector table, configuring the timer,
    /// writing the spurious interrupt vector, and sending an End-of-Interrupt (EOI).
    fn init(&self) {
        self.init_vector_table();
        self.setup_timer(
            LAPIC_TIMER_VECTOR,
            LAPIC_TIMER_INITIAL_COUNT,
            LAPIC_TIMER_DIVIDE_CONFIG,
        );
        self.write_spurious_interrupt_vector();
        self.eoi(); // Send an End-of-Interrupt (EOI) signal
        self.write_register(LVT_TPR, 0); // Set the Task Priority Register to 0
    }

    /// Configures the Local Vector Table (LVT) by masking all LAPIC interrupts.
    fn init_vector_table(&self) {
        // Mask all interrupts
        self.write_register(LVT_TIMER, APIC_LVT_MASKED); // Mask the timer interrupt
        self.write_register(LVT_THERMAL, APIC_LVT_MASKED); // Mask the thermal sensor interrupt
        self.write_register(LVT_PERFMON, APIC_LVT_MASKED); // Mask the performance monitoring interrupt
        self.write_register(LVT_LINT0, APIC_LVT_MASKED); // Mask the local interrupt 0
        self.write_register(LVT_LINT1, APIC_LVT_MASKED); // Mask the local interrupt 1
        self.write_register(LVT_ERROR, APIC_LVT_MASKED); // Mask the error interrupt
    }

    /// Configures the LAPIC timer with a specific vector, initial count, and divide configuration.
    fn setup_timer(&self, vector: u32, initial_count: u32, divide_config: u32) {
        // Set the timer divide configuration
        self.write_register(TIMER_DIVIDE_CONFIG_REG, divide_config);

        // Set the initial count for the timer
        self.write_register(TIMER_INITIAL_COUNT_REG, initial_count);

        // Configure the timer as periodic and set the interrupt vector
        let timer_lvt_value = vector | APIC_TIMER_PERIODIC_MODE;
        self.write_register(LVT_TIMER, timer_lvt_value);
    }

    /// Writes the spurious interrupt vector to enable LAPIC interrupts.
    fn write_spurious_interrupt_vector(&self) {
        let value = self.read_register(LVT_SPV) | APIC_SPURIOUS_INTERRUPT_VECTOR_ENABLE;
        self.write_register(LVT_SPV, value);
    }

    /// Writes a value to a LAPIC register
    fn write_register(&self, reg: u32, value: u32) {
        unsafe {
            // Calculate the register address
            let reg_ptr = self.local_apic_address.add(reg as usize / 4); // Divide by 4 because the base is a pointer to u32

            // Write the value to the register using volatile to ensure the write is not optimized out
            core::ptr::write_volatile(reg_ptr, value);
        }
    }

    /// Reads a value from a LAPIC register
    fn read_register(&self, reg: u32) -> u32 {
        unsafe {
            // Calculate the register address
            let reg_ptr = self.local_apic_address.add(reg as usize / 4); // Divide by 4 because the base is a pointer to u32

            // Read the value from the register using volatile to ensure the read is not optimized out
            core::ptr::read_volatile(reg_ptr)
        }
    }
}
