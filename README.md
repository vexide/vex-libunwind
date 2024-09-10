# vex-libunwind

> Idiomatic Rust bindings for LLVM libunwind on VEX V5 robots

## Install

```
cargo add --git https://github.com/vexide/vex-libunwind.git
```

## Usage

To unwind from the current execution point, also known as "local" unwinding, capture the current CPU state with `UnwindContext` and then step through each stack frame with an `UnwindCursor`.

```rs
let context = UnwindContext::new().unwrap();
let mut cursor = UnwindCursor::new(&context);

loop {
    // Print instruction pointer (i.e. "program counter")
    println!("{:?}", cursor.register(registers::UNW_REG_IP));

    if !cursor.step().unwrap() {
        // End of stack reached
        break;
    }
}
```

### Prerequisites

Libunwind will not find stack frames unless you add unwind tables to your program and show it where to find them.

Add unwind tables by updating your target file or using the `-Cforce-unwind-tables=on` rust flag:

```json
{
    ...,
    "default-uwtable": true
}
```

Then update your target file to include the unwind tables and the symbols libunwind uses to find them:

```ld
SECTIONS {
    /* ... */

    .eh_frame : {
        __eh_frame_start = .;
       *(.eh_frame)
        __eh_frame_end = .;
    } > RAM

    .eh_framehdr : {
       __eh_frame_hdr_start = .;
       *(.eh_framehdr)
       __eh_frame_hdr_end = .;
    } > RAM

    .ARM.exidx : {
        __exidx_start = .;
        *(.ARM.exidx*)
        *(.gnu.linkonce.armexidix.*.*)
        __exidx_end = .;
    } > RAM

    /* ... */
}
```

### Further Reading

Documentation for LLVM-flavored libunwind: <https://github.com/llvm/llvm-project/blob/main/libunwind/docs/index.rst>

Documentation for similar but distinct libunwind/libunwind project:

- <https://www.nongnu.org/libunwind/man/libunwind(3).html>
- <https://github.com/libunwind/libunwind>
