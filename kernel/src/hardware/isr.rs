use core::arch::asm;

use super::{
    idt::{set_idt, set_idt_gate},
    port_io::byte_out,
};

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Registers {
    pub ds: u64,       // Data segment selector
    pub rdi: u64,      // Source index
    pub rsi: u64,      // Destination index
    pub rbp: u64,      // Base pointer
    pub rsp: u64,      // Stack pointer
    pub rdx: u64,      // Data register
    pub rcx: u64,      // Counter register
    pub rbx: u64,      // Base register
    pub rax: u64,      // Accumulator register
    pub int_no: u64,   // Interrupt number
    pub err_code: u64, // Error code
    pub rip: u64,      // Instruction pointer
    pub cs: u64,       // Code segment selector
    pub rflags: u64,   // Flags register
    pub user_rsp: u64, // User stack pointer
    pub ss: u64,       // Stack segment selector
}

static EXCEPTION_MESSAGES: [&'static str; 32] = [
    "Division by zero exception",
    "Debug exception",
    "Non maskable interrupt",
    "Breakpoint exception",
    "Into detected overflow",
    "Out of bounds exception",
    "Invalid opcode exception",
    "No coprocessor exception",
    "Double fault (pushes an error code)",
    "Coprocessor segment overrun",
    "Bad TSS (pushes an error code)",
    "Segment not present (pushes an error code)",
    "Stack fault (pushes an error code)",
    "General protection fault (pushes an error code)",
    "Page fault (pushes an error code)",
    "Unknown interrupt exception",
    "Coprocessor fault",
    "Alignment check exception",
    "Machine check exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
];

extern "C" {
    fn isr_0();
    fn isr_1();
    fn isr_2();
    fn isr_3();
    fn isr_4();
    fn isr_5();
    fn isr_6();
    fn isr_7();
    fn isr_8();
    fn isr_9();
    fn isr_10();
    fn isr_11();
    fn isr_12();
    fn isr_13();
    fn isr_14();
    fn isr_15();
    fn isr_16();
    fn isr_17();
    fn isr_18();
    fn isr_19();
    fn isr_20();
    fn isr_21();
    fn isr_22();
    fn isr_23();
    fn isr_24();
    fn isr_25();
    fn isr_26();
    fn isr_27();
    fn isr_28();
    fn isr_29();
    fn isr_30();
    fn isr_31();

    fn irq_0();
    fn irq_1();
    fn irq_2();
    fn irq_3();
    fn irq_4();
    fn irq_5();
    fn irq_6();
    fn irq_7();
    fn irq_8();
    fn irq_9();
    fn irq_10();
    fn irq_11();
    fn irq_12();
    fn irq_13();
    fn irq_14();
    fn irq_15();
}

pub fn isr_install() {
    set_idt_gate(0, isr_0 as u64);
    set_idt_gate(1, isr_1 as u64);
    set_idt_gate(2, isr_2 as u64);
    set_idt_gate(3, isr_3 as u64);
    set_idt_gate(4, isr_4 as u64);
    set_idt_gate(5, isr_5 as u64);
    set_idt_gate(6, isr_6 as u64);
    set_idt_gate(7, isr_7 as u64);
    set_idt_gate(8, isr_8 as u64);
    set_idt_gate(9, isr_9 as u64);
    set_idt_gate(10, isr_10 as u64);
    set_idt_gate(11, isr_11 as u64);
    set_idt_gate(12, isr_12 as u64);
    set_idt_gate(13, isr_13 as u64);
    set_idt_gate(14, isr_14 as u64);
    set_idt_gate(15, isr_15 as u64);
    set_idt_gate(16, isr_16 as u64);
    set_idt_gate(17, isr_17 as u64);
    set_idt_gate(18, isr_18 as u64);
    set_idt_gate(19, isr_19 as u64);
    set_idt_gate(20, isr_20 as u64);
    set_idt_gate(21, isr_21 as u64);
    set_idt_gate(22, isr_22 as u64);
    set_idt_gate(23, isr_23 as u64);
    set_idt_gate(24, isr_24 as u64);
    set_idt_gate(25, isr_25 as u64);
    set_idt_gate(26, isr_26 as u64);
    set_idt_gate(27, isr_27 as u64);
    set_idt_gate(28, isr_28 as u64);
    set_idt_gate(29, isr_29 as u64);
    set_idt_gate(30, isr_30 as u64);
    set_idt_gate(31, isr_31 as u64);

    byte_out(0x20, 0x11);
    byte_out(0xA0, 0x11);
    byte_out(0x21, 0x20);
    byte_out(0xA1, 0x28);
    byte_out(0x21, 0x04);
    byte_out(0xA1, 0x02);
    byte_out(0x21, 0x01);
    byte_out(0xA1, 0x01);
    byte_out(0x21, 0x0);
    byte_out(0xA1, 0x0);

    set_idt_gate(32, irq_0 as u64);
    set_idt_gate(33, irq_1 as u64);
    set_idt_gate(34, irq_2 as u64);
    set_idt_gate(35, irq_3 as u64);
    set_idt_gate(36, irq_4 as u64);
    set_idt_gate(37, irq_5 as u64);
    set_idt_gate(38, irq_6 as u64);
    set_idt_gate(39, irq_7 as u64);
    set_idt_gate(40, irq_8 as u64);
    set_idt_gate(41, irq_9 as u64);
    set_idt_gate(42, irq_10 as u64);
    set_idt_gate(43, irq_11 as u64);
    set_idt_gate(44, irq_12 as u64);
    set_idt_gate(45, irq_13 as u64);
    set_idt_gate(46, irq_14 as u64);
    set_idt_gate(47, irq_15 as u64);

    set_idt();

    unsafe {
        asm!("sti");
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: Registers) {
    let mut vga = crate::drivers::vga::VgaWriter::new();

    vga.move_cursor(0, 24);

    let color_code = crate::drivers::vga::ColorCode::new(
        crate::drivers::vga::Color::Wht,
        crate::drivers::vga::Color::Blu,
    );

    vga.write("\nBreakpoint ", color_code);
    vga.write_num(stack_frame.int_no as u32, color_code);
}

extern "x86-interrupt" fn fpu_error_handler(stack_frame: &mut Registers) {
    let mut vga = crate::drivers::vga::VgaWriter::new();

    vga.move_cursor(0, 24);

    let color_code = crate::drivers::vga::ColorCode::new(
        crate::drivers::vga::Color::Wht,
        crate::drivers::vga::Color::Blu,
    );

    vga.write("\nFPU error ", color_code);
    vga.write_num(stack_frame.int_no as u32, color_code);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: Registers, _error_code: u64) -> ! {
    let mut vga = crate::drivers::vga::VgaWriter::new();

    vga.move_cursor(0, 24);

    let color_code = crate::drivers::vga::ColorCode::new(
        crate::drivers::vga::Color::Wht,
        crate::drivers::vga::Color::Blu,
    );

    vga.write("\nDouble fault ", color_code);
    vga.write_num(stack_frame.int_no as u32, color_code);

    loop {}
}

extern "x86-interrupt" fn device_not_available_handler(stack_frame: Registers) -> ! {
    let mut vga = crate::drivers::vga::VgaWriter::new();

    vga.move_cursor(0, 24);

    let color_code = crate::drivers::vga::ColorCode::new(
        crate::drivers::vga::Color::Wht,
        crate::drivers::vga::Color::Blu,
    );

    vga.write("\nDevice not available ", color_code);
    vga.write_num(stack_frame.int_no as u32, color_code);

    loop {}
}

#[no_mangle]
pub extern "C" fn int_handler(regs: Registers) {
    // if regs.int_no >= 40 {
    //     byte_out(0xA0, 0x20);
    // }
    // byte_out(0x20, 0x20);

    let mut vga = crate::drivers::vga::VgaWriter::new();

    vga.move_cursor(0, 24);

    let color_code = crate::drivers::vga::ColorCode::new(
        crate::drivers::vga::Color::Wht,
        crate::drivers::vga::Color::Gry,
    );

    vga.write("Received Interrupt ", color_code);
    vga.write_num(regs.int_no as u32, color_code);
    vga.write("\nMsg: ", color_code);
    let err = regs.err_code;
    vga.write(" with error code: ", color_code);
    vga.write_num(err as u32, color_code);

    let message = EXCEPTION_MESSAGES[regs.int_no as usize];
    vga.write(message, color_code);

    vga.write("\n", color_code);
    vga.write("RAX: ", color_code);
    vga.write_num(regs.rax as u32, color_code);
    vga.write("\n", color_code);
}

#[no_mangle]
pub extern "C" fn irq_handler(regs: Registers) {
    if regs.int_no >= 40 {
        byte_out(0xA0, 0x20);
    }
    byte_out(0x20, 0x20);

    let mut vga = crate::drivers::vga::VgaWriter::new();

    vga.move_cursor(0, 24);

    let color_code = crate::drivers::vga::ColorCode::new(
        crate::drivers::vga::Color::Wht,
        crate::drivers::vga::Color::Blu,
    );

    vga.write("\nIRQ ", color_code);
    vga.write_num(regs.int_no as u32, color_code);
}
