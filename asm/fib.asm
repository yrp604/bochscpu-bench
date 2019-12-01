[bits 64]

_start:
    push 0
    push 0
    push 1

loop:
    pop rax
    pop rbx
    pop rcx

    mov rdx, rax
    add rax, rbx
    mov rbx, rdx

    inc rcx

    push rcx
    push rbx
    push rax

    cmp rcx, 0xffffff
    jne loop

    nop
