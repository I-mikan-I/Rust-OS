# Rust Risc-V Operating System

This minimal operating system showcases using Rust as a systems-programming language and concepts such as
memory management, process management and interrupt handling.

## Dev-Dependencies

- Rust toolchain: riscv64gc-unknown-none-elf nightly
- qemu-system-riscv64
- [opt] gdb-multiarch

## Running

```sh
make run
```

## Debug using gdb-multiarch

```sh
make debug
gdb-multiarch /path/to/elf
(gdb) target remote :3333
```