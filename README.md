# bochscpu-bench

# usage

1. Clone bochscpu
2. Install the binary artifacts from bochscpu-build releases
3. Clone this repo adjacent to your bochscpu checkout, E.g.:
```
$ ls -l
total 0
drwxrwxrwx 1 x x 4096 Jan  3 22:58 bochscpu
drwxrwxrwx 1 x x 4096 Jan  2 23:37 bochscpu-bench
```
4. `cargo run --release` from the bochscpu-bench directory. You should see
output similar to the following:

```
   Compiling bochscpu-benches v0.1.0 (C:\Users\x\Documents\bochscpu\bochscpu-bench)
    Finished release [optimized] target(s) in 0.91s
     Running `target\release\fib.exe`
creating a mapping from gva 0x41410000 to gpa 0x81810000 for the text...
creating a mapping from gva 0x12345000 to gpa 0x67890000 for the stack...
mapping gpa 0x81810000 to hva 0x1baec2b0000...
mapping gpa 0x12345000 to hva 0x1baec2c0000...
page table serialized, base @ 0x0
mapping page table gpa 0x0 to hva 0x1baec2d0000...
mapping page table gpa 0x1000 to hva 0x1baec2e0000...
mapping page table gpa 0x2000 to hva 0x1baec8d0000...
mapping page table gpa 0x3000 to hva 0x1baec8e0000...
mapping page table gpa 0x4000 to hva 0x1baec8f0000...
mapping page table gpa 0x5000 to hva 0x1baec900000...
setting up cpu registers...
rax=0000000000000000 rbx=0000000000000000 rcx=0000000000000000
rdx=0000000000000000 rsi=0000000000000000 rdi=0000000000000000
rip=0000000041410000 rsp=0000000012345800 rbp=0000000000000000
 r8=0000000000000000  r9=0000000000000000 r10=0000000000000000
r11=0000000000000000 r12=0000000000000000 r13=0000000000000000
r14=0000000000000000 r15=0000000000000000
writing 34 bytes of fib code to gva 0x41410000...
setting up emulation hooks...
done, starting emulation...
result in rax is afe41bcaba69f23b, 16777215 loops
emulated 201326584 ins with 50331645 mem reads and 50331648 mem writes in 2.5824578s, 100.66m ips
```

## fib bench

This is a dumb program to execute a tight loop of assembly. It completely
integer wrapping, and makes un-needed memory writes intentionally.

```
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
```
