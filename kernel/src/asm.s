.section .bss
.balign 2
.lcomm kernel_stack, 1024 * 1024

.section .text
.global kernel_main
kernel_main:
    mov rsp, OFFSET kernel_stack + 1024 * 1024
    call kernel_main_new_stack
.fin:
    hlt
    jmp .fin
