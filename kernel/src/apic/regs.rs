pub const LAPIC_ID: u32 = 0x20;
pub const LAPIC_VERSION: u32 = 0x30;
pub const LAPIC_TASK_PRIORITY: u32 = 0x80;
pub const LAPIC_ARBITRATION_PRIORITY: u32 = 0x90;
pub const LAPIC_PROCESSOR_PRIORITY: u32 = 0xa0;
pub const LAPIC_EOI: u32 = 0xb0;
pub const LAPIC_REMOTE_READ: u32 = 0xc0;
pub const LAPIC_LOGICAL_DESTINATION: u32 = 0xd0;
pub const LAPIC_DESTINATION_FORMAT: u32 = 0xe0;
pub const LAPIC_SPURIOUS_INTERRUPT_VECTOR: u32 = 0xf0;
pub const LAPIC_IN_SERVICE_0: u32 = 0x100;
pub const LAPIC_TRIGGER_MODE_0: u32 = 0x180;
pub const LAPIC_INTERRUPT_REQUEST_0: u32 = 0x200;
pub const LAPIC_ERROR_STATUS: u32 = 0x280;
pub const LAPIC_INTERRUPT_COMMAND_LOW: u32 = 0x300;
pub const LAPIC_INTERRUPT_COMMAND_HIGH: u32 = 0x310;
pub const LAPIC_LVT_TIMER: u32 = 0x320;
pub const LAPIC_LVT_THERMAL_SENSOR: u32 = 0x330;
pub const LAPIC_LVT_PERFORMANCE_MONITORING_COUNTERS: u32 = 0x340;
pub const LAPIC_LVT_LINT0: u32 = 0x350;
pub const LAPIC_LVT_LINT1: u32 = 0x360;
pub const LAPIC_LVT_ERROR: u32 = 0x370;
pub const LAPIC_INITIAL_COUNT: u32 = 0x380;
pub const LAPIC_CURRENT_COUNT: u32 = 0x390;
pub const LAPIC_DIVIDE_CONFIGURATION: u32 = 0x3e0;