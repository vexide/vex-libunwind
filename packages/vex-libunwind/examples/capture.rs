#![allow(missing_docs)]

use vex_libunwind::{registers, UnwindContext, UnwindCursor};
use vex_sdk_jumptable as _;

#[inline(never)]
fn main() {
    call_capture();
}

#[inline(never)]
fn call_capture() {
    capture_backtrace();
}

#[inline(never)]
fn capture_backtrace() {
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
    .unwrap();
}
