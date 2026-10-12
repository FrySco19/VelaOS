; VelaOS x86_64 boot stub: Multiboot2 header, 32-bit entry, identity-map 4 GiB, enter long mode.
bits 32
section .multiboot2
align 8
mb2_start:
    dd 0xE85250D6                 ; magic
    dd 0                          ; architecture: i386 protected mode
    dd mb2_end - mb2_start
    dd -(0xE85250D6 + 0 + (mb2_end - mb2_start))
align 8
    ; framebuffer request tag: 1024x768x32
    dw 5, 0
    dd 20
    dd 1024, 768, 32
align 8
    dw 0, 0                       ; end tag
    dd 8
mb2_end:

section .bss
align 4096
pml4:   resb 4096
pdpt:   resb 4096
pd:     resb 4096*4
stack_bottom: resb 65536
stack_top:

section .rodata
align 8
gdt64:
    dq 0
    dq 0x00AF9A000000FFFF         ; 64-bit code
    dq 0x00CF92000000FFFF         ; data
gdt64_ptr:
    dw $ - gdt64 - 1
    dq gdt64

section .text
global _start
extern kmain
_start:
    cli
    mov esp, stack_top
    mov edi, eax                  ; multiboot2 magic
    mov esi, ebx                  ; multiboot2 info pointer

    ; PML4[0] -> PDPT, PDPT[0..3] -> PD[0..3], each PD maps 1 GiB with 2 MiB pages
    mov eax, pdpt
    or eax, 3
    mov [pml4], eax
    xor ecx, ecx
.pdpt_loop:
    mov eax, ecx
    shl eax, 12
    add eax, pd
    or eax, 3
    mov [pdpt + ecx*8], eax
    inc ecx
    cmp ecx, 4
    jne .pdpt_loop
    xor ecx, ecx
.pd_loop:                          ; 2048 entries * 2 MiB = 4 GiB
    mov eax, ecx
    shl eax, 21
    or eax, 0x83                  ; present | writable | huge
    mov [pd + ecx*8], eax
    mov eax, ecx
    shr eax, 11                   ; high dword of physical address
    mov [pd + ecx*8 + 4], eax
    inc ecx
    cmp ecx, 2048
    jne .pd_loop

    mov eax, pml4
    mov cr3, eax
    mov eax, cr4
    or eax, (1 << 5)              ; PAE
    mov cr4, eax
    mov ecx, 0xC0000080           ; EFER
    rdmsr
    or eax, (1 << 8)              ; LME
    wrmsr
    mov eax, cr0
    or eax, (1 << 31) | 1         ; PG | PE
    mov cr0, eax
    lgdt [gdt64_ptr]
    jmp 0x08:long_mode

bits 64
long_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    mov rsp, stack_top
    ; edi/esi still hold magic/info (zero-extended)
    mov edi, edi
    mov esi, esi
    call kmain
.hang:
    hlt
    jmp .hang
