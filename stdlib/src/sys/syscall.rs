use core::arch::asm;

// Executes a system call with the given number and no arguments.

pub unsafe fn syscall0(n: usize) -> usize {
    let res: usize;
    asm!(
        "int 0x80",
        in("rax") n, // syscall number
        lateout("rax") res
    );
    res
}

pub unsafe fn syscall1(n: usize, arg1: usize) -> usize {
    let res: usize;
    asm!(
        "int 0x80",
        in("rax") n, // syscall number
        in("rdi") arg1, // first argument
        lateout("rax") res
    );
    res
}

pub unsafe fn syscall2(n: usize, arg1: usize, arg2: usize) -> usize {
    let res: usize;
    asm!(
        "int 0x80",
        in("rax") n, // syscall number
        in("rdi") arg1, // first argument
        in("rsi") arg2, // second argument
        lateout("rax") res
    );
    res
}

#[doc(hidden)]
pub unsafe fn syscall3(n: usize, arg1: usize, arg2: usize, arg3: usize) -> usize {
    let res: usize;
    asm!(
        "int 0x80",
        in("rax") n, // syscall number
        in("rdi") arg1, // first argument
        in("rsi") arg2, // second argument
        in("r10") arg3, // third argument
        lateout("rax") res
    );
    res
}
