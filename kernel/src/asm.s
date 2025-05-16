.section .bss
.balign 16
.lcomm kernel_stack, 1024 * 1024

.section .text
.balign 16
.global kernel_main
kernel_main:
    lea rsp, kernel_stack[1024 * 1024 + rip]
    call kernel_main_new_stack
.fin:
    hlt
    jmp .fin
