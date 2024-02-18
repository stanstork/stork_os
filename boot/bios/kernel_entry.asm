[bits 64]           
[extern _start]     
call _start         ; invoke _start() in our Rust kernel
jmp $               ; Hang forever when we return from the kernel