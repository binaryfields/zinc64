// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

// SPEC: AArch64 Reference Manual: D12.2.36 ESR_EL1, Exception Syndrome Register (EL1), p. 2770

use super::UART;
use cortex_a::asm;
use register::{mmio::ReadOnly, register_bitfields};

global_asm!(include_str!("vectors.S"));

register_bitfields! {u64,
    /// Exception Syndrome Register
    ESR_EL1 [
        /// Instruction Specific Syndrome
        ISS OFFSET(0) NUMBITS(24) [],
        /// Exception class
        EC OFFSET(26) NUMBITS(6) [
            Unknown = 0b000000,
            TrappedWFIorWFE = 0b000001,
            IllegalExecution = 0b001110,
            SystemCall = 0b010101,
            InstructionAbortLowerEL = 0b100000,
            InstructionAbortSameEL = 0b100001,
            InstructionAlignment = 0b100010,
            DataAbortLowerEL = 0b100100,
            DataAbortSameEL = 0b100101,
            StackAlignment = 0b100110,
            FloatingPoint = 0b101100
        ]
    ]
}

#[repr(C)]
pub struct ExceptionContext {
    gpr: [u64; 31],
    spsr_el1: u64,
    elr_el1: u64,
    esr_el1: ReadOnly<u64, ESR_EL1::Register>,
}

#[no_mangle]
unsafe extern "C" fn default_exception_handler() {
    UART.puts("Unexpected exception. Halting CPU.\n");
    loop {
        asm::wfe()
    }
}

#[no_mangle]
unsafe extern "C" fn current_elx_synchronous(ec: &mut ExceptionContext) {
    let class = match ec.esr_el1.read_as_enum(ESR_EL1::EC) {
        Some(ESR_EL1::EC::Value::TrappedWFIorWFE) => "TrappedWFIorWFE",
        Some(ESR_EL1::EC::Value::IllegalExecution) => "IllegalExecution",
        Some(ESR_EL1::EC::Value::SystemCall) => "SystemCall",
        Some(ESR_EL1::EC::Value::InstructionAbortLowerEL) => "InstructionAbortLowerEL",
        Some(ESR_EL1::EC::Value::InstructionAbortSameEL) => "InstructionAbortSameEL",
        Some(ESR_EL1::EC::Value::InstructionAlignment) => "InstructionAlignment",
        Some(ESR_EL1::EC::Value::DataAbortLowerEL) => "DataAbortLowerEL",
        Some(ESR_EL1::EC::Value::DataAbortSameEL) => "DataAbortSameEL",
        Some(ESR_EL1::EC::Value::StackAlignment) => "StackAlignment",
        Some(ESR_EL1::EC::Value::FloatingPoint) => "FloatingPoint",
        _ => "Unknown",
    };
    let cause  = match ec.esr_el1.read_as_enum(ESR_EL1::EC) {
        Some(ESR_EL1::EC::Value::DataAbortLowerEL) | Some(ESR_EL1::EC::Value::DataAbortSameEL) => {
            let fault = match ec.esr_el1.read(ESR_EL1::ISS) >> 2 & 0x03 {
                0 => Some("Address size fault"),
                1 => Some("Translation fault"),
                2 => Some("Access flag fault"),
                3 => Some("Permission fault"),
                _ => None
            };
            if let Some(fault) = fault {
                let level = match ec.esr_el1.read(ESR_EL1::ISS) & 0x03 {
                    0 => "Level 0",
                    1 => "Level 1",
                    2 => "Level 2",
                    3 => "Level 3",
                    _ => "Invalid",
                };
                Some((fault, level))
            } else {
                None
            }
        },
        _ => None,
    };
    UART.puts("Synchronous exception: ");
    UART.puts(class);
    UART.puts("\n");
    if let Some(cause) = cause {
        UART.puts("    Cause: ");
        UART.puts(cause.0);
        UART.puts(", ");
        UART.puts(cause.1);
        UART.puts("\n");
    }
    print_reg("    ESR_EL1", ec.esr_el1.get());
    print_reg("    ELR_EL1", ec.elr_el1);
    print_reg("    SPSR_EL1", ec.spsr_el1);
    loop {
        asm::wfe()
    }
    // asm! { "RESTORE_CONTEXT" }
    // asm::eret();
}

#[inline]
fn print_reg(name: &str, value: u64) {
    UART.puts(name);
    UART.puts(": 0x");
    UART.hex(value);
    UART.puts("\n");
}
