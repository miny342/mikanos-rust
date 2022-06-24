.section .bss
.balign 2
.lcomm kernel_stack, 1024 * 1024

.section .text
.global kernel_main
kernel_main:
    lea rsp, kernel_stack[1024 * 1024 + rip]
    call kernel_main_new_stack
.fin:
    hlt
    jmp .fin

.global set_csss
set_csss:
    mov ss, si
    lea rax, .next[rip]
    push rdi
    push rax
    retfq
.next:
    ret

