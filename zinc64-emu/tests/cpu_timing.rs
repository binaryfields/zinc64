// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use zinc64_core::{Addressable, Cpu, IoPort, IrqLine, Pin, Ram, TickFn};
use zinc64_emu::cpu::Cpu6510;

struct MockMemory {
    ram: Ram,
}

impl MockMemory {
    pub fn new(ram: Ram) -> Self {
        MockMemory { ram }
    }
}

impl Addressable for MockMemory {
    fn read(&self, address: u16) -> u8 {
        self.ram.read(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.ram.write(address, value);
    }
}

fn setup_cpu() -> Cpu6510 {
    let ba_line = Rc::new(RefCell::new(Pin::new_high()));
    let cpu_io_port = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
    let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
    let cpu_nmi = Rc::new(RefCell::new(IrqLine::new("nmi")));
    let mem = Rc::new(RefCell::new(MockMemory::new(Ram::new(0x10000))));
    Cpu6510::new(mem, cpu_io_port, ba_line, cpu_irq, cpu_nmi)
}

// Based on 65xx Processor Data from http://www.romhacking.net/documents/318/

const OPCODE_TIMING: [u8; 256] = [
    7, // 00 BRK #$ab
    6, // 01 ORA ($ab,X)
    0, // 02 HLT*
    0, // 03 ASO* ($ab,X)
    0, // 04 SKB* $ab
    3, // 05 ORA $ab
    5, // 06 ASL $ab
    0, // 07 ASO* $ab
    3, // 08 PHP
    2, // 09 ORA #$ab
    2, // 0A ASL A
    0, // 0B ANC* #$ab
    0, // 0C SKW* $abcd
    4, // 0D ORA $abcd
    6, // 0E ASL $abcd
    0, // 0F ASO* $abcd
    2, // 10 BPL nearlabel
    5, // 11 ORA ($ab),Y
    0, // 12 HLT*
    0, // 13 ASO* ($ab),Y
    0, // 14 SKB* $ab,X
    4, // 15 ORA $ab,X
    6, // 16 ASL $ab,X
    0, // 17 ASO* $ab,X
    2, // 18 CLC
    4, // 19 ORA $abcd,Y
    0, // 1A NOP*
    0, // 1B ASO* $abcd,Y
    0, // 1C SKW* $abcd,X
    4, // 1D ORA $abcd,X
    7, // 1E ASL $abcd,X
    0, // 1F ASO* $abcd,X
    6, // 20 JSR $abcd
    6, // 21 AND ($ab,X)
    0, // 22 HLT*
    0, // 23 RLA* ($ab,X)
    3, // 24 BIT $ab
    3, // 25 AND $ab
    5, // 26 ROL $ab
    0, // 27 RLA* $ab
    4, // 28 PLP
    2, // 29 AND #$ab
    2, // 2A ROL A
    0, // 2B ANC* #$ab
    4, // 2C BIT $abcd
    4, // 2D AND $abcd
    6, // 2E ROL $abcd
    0, // 2F RLA* $abcd
    2, // 30 BMI nearlabel
    5, // 31 AND ($ab),Y
    0, // 32 HLT*
    0, // 33 RLA* ($ab),Y
    0, // 34 SKB* $ab,X
    4, // 35 AND $ab,X
    6, // 36 ROL $ab,X
    0, // 37 RLA* $ab,X
    2, // 38 SEC
    4, // 39 AND $abcd,Y
    0, // 3A NOP*
    0, // 3B RLA* $abcd,Y
    0, // 3C SKW* $abcd,X
    4, // 3D AND $abcd,X
    7, // 3E ROL $abcd,X
    0, // 3F RLA* $abcd,X
    6, // 40 RTI
    6, // 41 EOR ($ab,X)
    0, // 42 HLT*
    8, // 43 LSE* ($ab,X)
    0, // 44 SKB* $ab
    3, // 45 EOR $ab
    5, // 46 LSR $ab
    5, // 47 LSE* $ab
    3, // 48 PHA
    2, // 49 EOR #$ab
    2, // 4A LSR A
    2, // 4B ALR* #$ab
    3, // 4C JMP $abcd
    4, // 4D EOR $abcd
    6, // 4E LSR $abcd
    6, // 4F LSE* $abcd
    2, // 50 BVC nearlabel
    5, // 51 EOR ($ab),Y
    0, // 52 HLT*
    8, // 53 LSE* ($ab),Y
    0, // 54 SKB* $ab,X
    4, // 55 EOR $ab,X
    6, // 56 LSR $ab,X
    6, // 57 LSE* $ab,X
    2, // 58 CLI
    4, // 59 EOR $abcd,Y
    0, // 5A NOP*
    7, // 5B LSE* $abcd,Y
    0, // 5C SKW* $abcd,X
    4, // 5D EOR $abcd,X
    7, // 5E LSR $abcd,X
    7, // 5F LSE* $abcd,X
    6, // 60 RTS
    6, // 61 ADC ($ab,X)
    0, // 62 HLT*
    0, // 63 RRA* ($ab,X)
    0, // 64 SKB* $ab
    3, // 65 ADC $ab
    5, // 66 ROR $ab
    0, // 67 RRA* $ab
    4, // 68 PLA
    2, // 69 ADC #$ab
    2, // 6A ROR A
    0, // 6B ARR* #$ab
    5, // 6C JMP ($abcd)
    4, // 6D ADC $abcd
    6, // 6E ROR $abcd
    0, // 6F RRA* $abcd
    2, // 70 BVS nearlabel
    5, // 71 ADC ($ab),Y
    0, // 72 HLT*
    0, // 73 RRA* ($ab),Y
    0, // 74 SKB* $ab,X
    4, // 75 ADC $ab,X
    6, // 76 ROR $ab,X
    0, // 77 RRA* $ab,X
    2, // 78 SEI
    4, // 79 ADC $abcd,Y
    0, // 7A NOP*
    0, // 7B RRA* $abcd,Y
    0, // 7C SKW* $abcd,X
    4, // 7D ADC $abcd,X
    7, // 7E ROR $abcd,X
    0, // 7F RRA* $abcd,X
    0, // 80 SKB* #$ab
    6, // 81 STA ($ab,X)
    0, // 82 SKB* #$ab
    0, // 83 SAX* ($ab,X)
    3, // 84 STY $ab
    3, // 85 STA $ab
    3, // 86 STX $ab
    0, // 87 SAX* $ab
    2, // 88 DEY
    0, // 89 SKB* #$ab
    2, // 8A TXA
    2, // 8B ANE* #$ab
    4, // 8C STY $abcd
    4, // 8D STA $abcd
    4, // 8E STX $abcd
    0, // 8F SAX* $abcd
    2, // 90 BCC nearlabel
    6, // 91 STA ($ab),Y
    0, // 92 HLT*
    0, // 93 SHA* ($ab),Y
    4, // 94 STY $ab,X
    4, // 95 STA $ab,X
    4, // 96 STX $ab,Y
    0, // 97 SAX* $ab,Y
    2, // 98 TYA
    5, // 99 STA $abcd,Y
    2, // 9A TXS
    0, // 9B SHS* $abcd,Y
    0, // 9C SHY* $abcd,X
    5, // 9D STA $abcd,X
    0, // 9E SHX* $abcd,Y
    0, // 9F SHA* $abcd,Y
    2, // A0 LDY #$ab
    6, // A1 LDA ($ab,X)
    2, // A2 LDX #$ab
    6, // A3 LAX* ($ab,X)
    3, // A4 LDY $ab
    3, // A5 LDA $ab
    3, // A6 LDX $ab
    3, // A7 LAX* $ab
    2, // A8 TAY
    2, // A9 LDA #$ab
    2, // AA TAX
    2, // AB ANX* #$ab
    4, // AC LDY $abcd
    4, // AD LDA $abcd
    4, // AE LDX $abcd
    4, // AF LAX* $abcd
    2, // B0 BCS nearlabel
    5, // B1 LDA ($ab),Y
    0, // B2 HLT*
    5, // B3 LAX* ($ab),Y
    4, // B4 LDY $ab,X
    4, // B5 LDA $ab,X
    4, // B6 LDX $ab,Y
    4, // B7 LAX* $ab,Y
    2, // B8 CLV
    4, // B9 LDA $abcd,Y
    2, // BA TSX
    0, // BB LAS* $abcd,Y
    4, // BC LDY $abcd,X
    4, // BD LDA $abcd,X
    4, // BE LDX $abcd,Y
    4, // BF LAX* $abcd,Y
    2, // C0 CPY #$ab
    6, // C1 CMP ($ab,X)
    0, // C2 SKB* #$ab
    0, // C3 DCM* ($ab,X)
    3, // C4 CPY $ab
    3, // C5 CMP $ab
    5, // C6 DEC $ab
    0, // C7 DCM* $ab
    2, // C8 INY
    2, // C9 CMP #$ab
    2, // CA DEX
    2, // CB SBX* #$ab
    4, // CC CPY $abcd
    4, // CD CMP $abcd
    6, // CE DEC $abcd
    0, // CF DCM* $abcd
    2, // D0 BNE nearlabel
    5, // D1 CMP ($ab),Y
    0, // D2 HLT*
    0, // D3 DCM* ($ab),Y
    0, // D4 SKB* $ab,X
    4, // D5 CMP $ab,X
    6, // D6 DEC $ab,X
    0, // D7 DCM* $ab,X
    2, // D8 CLD
    4, // D9 CMP $abcd,Y
    0, // DA NOP*
    0, // DB DCM* $abcd,Y
    0, // DC SKW* $abcd,X
    4, // DD CMP $abcd,X
    7, // DE DEC $abcd,X
    0, // DF DCM* $abcd,X
    2, // E0 CPX #$ab
    6, // E1 SBC ($ab,X)
    0, // E2 SKB* #$ab
    0, // E3 INS* ($ab,X)
    3, // E4 CPX $ab
    3, // E5 SBC $ab
    5, // E6 INC $ab
    0, // E7 INS* $ab
    2, // E8 INX
    2, // E9 SBC #$ab
    2, // EA NOP
    0, // EB SBC* #$ab
    4, // EC CPX $abcd
    4, // ED SBC $abcd
    6, // EE INC $abcd
    0, // EF INS* $abcd
    2, // F0 BEQ nearlabel
    5, // F1 SBC ($ab),Y
    0, // F2 HLT*
    0, // F3 INS* ($ab),Y
    0, // F4 SKB* $ab,X
    4, // F5 SBC $ab,X
    6, // F6 INC $ab,X
    0, // F7 INS* $ab,X
    2, // F8 SED
    4, // F9 SBC $abcd,Y
    0, // FA NOP*
    0, // FB INS* $abcd,Y
    0, // FC SKW* $abcd,X
    4, // FD SBC $abcd,X
    7, // FE INC $abcd,X
    0, // FF INS* $abcd,X
];

#[test]
fn opcode_timing() {
    let mut cpu = setup_cpu();
    for opcode in 0..256 {
        let cycles = OPCODE_TIMING[opcode];
        if cycles > 0 {
            let clock = Rc::new(Cell::new(0u8));
            let clock_clone = clock.clone();
            let tick_fn: TickFn = Rc::new(move || {
                clock_clone.set(clock_clone.get().wrapping_add(1));
            });
            cpu.write(0x1000, opcode as u8);
            cpu.write(0x1001, 0x00);
            cpu.write(0x1002, 0x10);
            cpu.set_pc(0x1000);
            cpu.step(&tick_fn);
            assert_eq!(
                cycles,
                clock.get(),
                "opcode {:02x} timing failed",
                opcode as u8
            );
        }
    }
}
