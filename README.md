# vex-libunwind

> Idiomatic Rust bindings for LLVM libunwind on VEX V5 robots

**Development status:** Passively maintained (There are no plans for new features, but the
maintainer intends to respond to issues that get filed.)

## Install

```
cargo add vex-libunwind
```

## Usage

To unwind from the current execution point, also known as "local" unwinding, capture the current CPU state with `UnwindContext` and then step through each stack frame with an `UnwindCursor`.

```rs
UnwindContext::capture(|ctx| {
    let mut cursor = UnwindCursor::new(&ctx)?;

    println!("Capturing backtrace...");

    loop {
        // Print instruction pointer (i.e. program counter)
        println!("{:?}", cursor.register(registers::UNW_REG_IP));

        if !cursor.step().unwrap() {
            // End of stack reached
            break;
        }
    }

    Ok(())
})
```

### Prerequisites

Libunwind will not find stack frames unless you add unwind tables to your program and show it where to find them.

If you are using the `armv7a-vex-v5` Rust target, this is already done for you.

### Further Reading

Documentation for LLVM-flavored libunwind: <https://github.com/llvm/llvm-project/blob/main/libunwind/docs/index.rst>

Documentation for similar but distinct libunwind/libunwind project:

- <https://www.nongnu.org/libunwind/man/libunwind(3).html>
- <https://github.com/libunwind/libunwind>
