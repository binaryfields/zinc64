// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

#[derive(Copy, Clone, Debug)]
pub enum MicroOp {
    FetchOpcode,
    FetchOpcodeDiscard,
    FetchOperand,
    FetchAdl,
    FetchAdh,
    IncrementAdlX,
    IncrementAdlY,
    IndirectFetchAdl,
    IndirectFetchAdh,
    ReadData,
    ReadDataOrFixAdh,
    WriteData,
    // Move (16)
    OpLDA,
    OpLDX,
    OpLDY,
    OpSTA,
    OpSTX,
    OpSTY,
    OpTAX,
    OpTXA,
    OpTAY,
    OpTYA,
    OpTSX,
    OpTXS,
    OpPLA,
    OpPLP,
    OpPHA,
    OpPHP,
    // Logical/Arithmetic (19)
    OpAND,
    OpEOR,
    OpORA,
    OpADC,
    OpSBC,
    OpBIT,
    OpCMP,
    OpCPX,
    OpCPY,
    OpDEC,
    OpDEX,
    OpDEY,
    OpINC,
    OpINX,
    OpINY,
    OpASL,
    OpASLImplied,
    OpLSR,
    OpLSRImplied,
    OpROL,
    OpROLImplied,
    OpROR,
    OpRORImplied,
    // Jump/Flag (21)
    OpBCC,
    OpBCS,
    OpBEQ,
    OpBNE,
    OpBMI,
    OpBPL,
    OpBVC,
    OpBVS,
    OpJMP,
    OpJSR,
    OpRTS,
    OpBRK,
    OpRTI,
    OpCLC,
    OpCLD,
    OpCLI,
    OpCLV,
    OpSEC,
    OpSED,
    OpSEI,
    OpNOP,
    // Undocumented
    OpANE,
    OpANX,
    OpALR,
    OpAXS,
    OpLAX,
    OpLSE,
    // Interrupts
    OpIRQ,
    OpNMI,
    OpRST,
}

#[derive(Copy, Clone)]
pub struct MicroOpPair(pub MicroOp, pub Option<MicroOp>);

impl MicroOpPair {
    pub const fn from(op: MicroOp) -> MicroOpPair {
        MicroOpPair(op, None)
    }

    pub const fn pair(op1: MicroOp, op2: MicroOp) -> MicroOpPair {
        MicroOpPair(op1, Some(op2))
    }
}

const fn implied(op: MicroOp) -> [MicroOpPair; 3] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn implied_3c(op: MicroOp) -> [MicroOpPair; 4] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn implied_4c(op: MicroOp) -> [MicroOpPair; 5] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
        MicroOpPair::from(op),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn implied_6c(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
        MicroOpPair::from(op),
        MicroOpPair::from(op),
        MicroOpPair::from(op),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn immediate(op: MicroOp) -> [MicroOpPair; 3] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchOperand),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn absolute_read(op: MicroOp) -> [MicroOpPair; 5] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::FetchAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn absolute_write(op: MicroOp) -> [MicroOpPair; 5] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::FetchAdh),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn absolute_rmw(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::FetchAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn absolutex_read(op: MicroOp) -> [MicroOpPair; 6] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::pair(MicroOp::FetchAdh, MicroOp::IncrementAdlX),
        MicroOpPair::from(MicroOp::ReadDataOrFixAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn absolutex_write(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::pair(MicroOp::FetchAdh, MicroOp::IncrementAdlX),
        MicroOpPair::from(MicroOp::ReadDataOrFixAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn absolutex_rmw(op: MicroOp) -> [MicroOpPair; 8] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::pair(MicroOp::FetchAdh, MicroOp::IncrementAdlX),
        MicroOpPair::from(MicroOp::FetchOpcodeDiscard), // FIXME
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn absolutey_read(op: MicroOp) -> [MicroOpPair; 6] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::pair(MicroOp::FetchAdh, MicroOp::IncrementAdlY),
        MicroOpPair::from(MicroOp::ReadDataOrFixAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn absolutey_write(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::pair(MicroOp::FetchAdh, MicroOp::IncrementAdlY),
        MicroOpPair::from(MicroOp::ReadDataOrFixAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn zeropage_read(op: MicroOp) -> [MicroOpPair; 4] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn zeropage_write(op: MicroOp) -> [MicroOpPair; 4] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn zeropage_rmw(op: MicroOp) -> [MicroOpPair; 6] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn zeropagex_read(op: MicroOp) -> [MicroOpPair; 5] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IncrementAdlX),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn zeropagex_write(op: MicroOp) -> [MicroOpPair; 5] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IncrementAdlX),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn zeropagex_rmw(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IncrementAdlX),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn zeropagey_read(op: MicroOp) -> [MicroOpPair; 5] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IncrementAdlY),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn zeropagey_write(op: MicroOp) -> [MicroOpPair; 5] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IncrementAdlY),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn indirectx_read(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IncrementAdlX),
        MicroOpPair::from(MicroOp::IndirectFetchAdl),
        MicroOpPair::from(MicroOp::IndirectFetchAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn indirectx_write(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IncrementAdlX),
        MicroOpPair::from(MicroOp::IndirectFetchAdl),
        MicroOpPair::from(MicroOp::IndirectFetchAdh),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn indirecty_read(op: MicroOp) -> [MicroOpPair; 7] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IndirectFetchAdl),
        MicroOpPair::pair(MicroOp::IndirectFetchAdh, MicroOp::IncrementAdlY),
        MicroOpPair::from(MicroOp::ReadDataOrFixAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn indirecty_write(op: MicroOp) -> [MicroOpPair; 8] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::IndirectFetchAdl),
        MicroOpPair::pair(MicroOp::IndirectFetchAdh, MicroOp::IncrementAdlY),
        MicroOpPair::from(MicroOp::ReadDataOrFixAdh),
        MicroOpPair::from(MicroOp::ReadData),
        MicroOpPair::pair(op, MicroOp::WriteData),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

const fn indirect(op: MicroOp) -> [MicroOpPair; 6] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchAdl),
        MicroOpPair::from(MicroOp::FetchAdh),
        MicroOpPair::from(MicroOp::IndirectFetchAdl),
        MicroOpPair::from(MicroOp::IndirectFetchAdh),
        MicroOpPair::pair(op, MicroOp::FetchOpcode),
    ]
}

const fn relative(op: MicroOp) -> [MicroOpPair; 4] {
    [
        MicroOpPair::from(MicroOp::FetchOpcode),
        MicroOpPair::from(MicroOp::FetchOperand),
        MicroOpPair::from(op),
        MicroOpPair::from(MicroOp::FetchOpcode),
    ]
}

static LDA_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpLDA);
static LDA_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpLDA);
static LDA_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpLDA);
static LDA_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpLDA);
static LDA_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpLDA);
static LDA_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpLDA);
static LDA_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpLDA);
static LDA_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpLDA);
static LDX_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpLDX);
static LDX_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpLDX);
static LDX_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpLDX);
static LDX_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpLDX);
static LDX_ZEROPAGEY: &[MicroOpPair] = &zeropagey_read(MicroOp::OpLDX);
static LDY_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpLDY);
static LDY_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpLDY);
static LDY_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpLDY);
static LDY_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpLDY);
static LDY_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpLDY);
static STA_ABSOLUTE: &[MicroOpPair] = &absolute_write(MicroOp::OpSTA);
static STA_ABSOLUTEX: &[MicroOpPair] = &absolutex_write(MicroOp::OpSTA);
static STA_ABSOLUTEY: &[MicroOpPair] = &absolutey_write(MicroOp::OpSTA);
static STA_INDIRECTX: &[MicroOpPair] = &indirectx_write(MicroOp::OpSTA);
static STA_INDIRECTY: &[MicroOpPair] = &indirecty_write(MicroOp::OpSTA);
static STA_ZEROPAGE: &[MicroOpPair] = &zeropage_write(MicroOp::OpSTA);
static STA_ZEROPAGEX: &[MicroOpPair] = &zeropagex_write(MicroOp::OpSTA);
static STX_ABSOLUTE: &[MicroOpPair] = &absolute_write(MicroOp::OpSTX);
static STX_ZEROPAGE: &[MicroOpPair] = &zeropage_write(MicroOp::OpSTX);
static STX_ZEROPAGEY: &[MicroOpPair] = &zeropagey_write(MicroOp::OpSTX);
static STY_ABSOLUTE: &[MicroOpPair] = &absolute_write(MicroOp::OpSTY);
static STY_ZEROPAGE: &[MicroOpPair] = &zeropage_write(MicroOp::OpSTY);
static STY_ZEROPAGEX: &[MicroOpPair] = &zeropagex_write(MicroOp::OpSTY);
static TAX_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpTAX);
static TAY_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpTAY);
static TSX_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpTSX);
static TXA_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpTXA);
static TXS_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpTXS);
static TYA_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpTYA);
static PHA_IMPLIED: &[MicroOpPair] = &implied_3c(MicroOp::OpPHA);
static PHP_IMPLIED: &[MicroOpPair] = &implied_3c(MicroOp::OpPHP);
static PLA_IMPLIED: &[MicroOpPair] = &implied_4c(MicroOp::OpPLA);
static PLP_IMPLIED: &[MicroOpPair] = &implied_4c(MicroOp::OpPLP);

static AND_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpAND);
static AND_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpAND);
static AND_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpAND);
static AND_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpAND);
static AND_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpAND);
static AND_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpAND);
static AND_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpAND);
static AND_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpAND);
static EOR_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpEOR);
static EOR_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpEOR);
static EOR_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpEOR);
static EOR_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpEOR);
static EOR_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpEOR);
static EOR_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpEOR);
static EOR_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpEOR);
static EOR_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpEOR);
static ORA_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpORA);
static ORA_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpORA);
static ORA_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpORA);
static ORA_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpORA);
static ORA_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpORA);
static ORA_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpORA);
static ORA_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpORA);
static ORA_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpORA);
static ADC_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpADC);
static ADC_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpADC);
static ADC_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpADC);
static ADC_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpADC);
static ADC_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpADC);
static ADC_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpADC);
static ADC_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpADC);
static ADC_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpADC);
static SBC_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpSBC);
static SBC_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpSBC);
static SBC_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpSBC);
static SBC_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpSBC);
static SBC_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpSBC);
static SBC_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpSBC);
static SBC_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpSBC);
static SBC_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpSBC);
static BIT_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpBIT);
static BIT_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpBIT);
static CMP_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpCMP);
static CMP_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpCMP);
static CMP_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpCMP);
static CMP_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpCMP);
static CMP_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpCMP);
static CMP_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpCMP);
static CMP_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpCMP);
static CMP_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpCMP);
static CPX_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpCPX);
static CPX_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpCPX);
static CPX_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpCPX);
static CPY_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpCPY);
static CPY_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpCPY);
static CPY_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpCPY);
static DEC_ABSOLUTE: &[MicroOpPair] = &absolute_rmw(MicroOp::OpDEC);
static DEC_ABSOLUTEX: &[MicroOpPair] = &absolutex_rmw(MicroOp::OpDEC);
static DEC_ZEROPAGE: &[MicroOpPair] = &zeropage_rmw(MicroOp::OpDEC);
static DEC_ZEROPAGEX: &[MicroOpPair] = &zeropagex_rmw(MicroOp::OpDEC);
static DEX_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpDEX);
static DEY_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpDEY);
static INC_ABSOLUTE: &[MicroOpPair] = &absolute_rmw(MicroOp::OpINC);
static INC_ABSOLUTEX: &[MicroOpPair] = &absolutex_rmw(MicroOp::OpINC);
static INC_ZEROPAGE: &[MicroOpPair] = &zeropage_rmw(MicroOp::OpINC);
static INC_ZEROPAGEX: &[MicroOpPair] = &zeropagex_rmw(MicroOp::OpINC);
static INX_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpINX);
static INY_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpINY);
static ASL_ABSOLUTE: &[MicroOpPair] = &absolute_rmw(MicroOp::OpASL);
static ASL_ABSOLUTEX: &[MicroOpPair] = &absolutex_rmw(MicroOp::OpASL);
static ASL_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpASLImplied);
static ASL_ZEROPAGE: &[MicroOpPair] = &zeropage_rmw(MicroOp::OpASL);
static ASL_ZEROPAGEX: &[MicroOpPair] = &zeropagex_rmw(MicroOp::OpASL);
static LSR_ABSOLUTE: &[MicroOpPair] = &absolute_rmw(MicroOp::OpLSR);
static LSR_ABSOLUTEX: &[MicroOpPair] = &absolutex_rmw(MicroOp::OpLSR);
static LSR_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpLSRImplied);
static LSR_ZEROPAGE: &[MicroOpPair] = &zeropage_rmw(MicroOp::OpLSR);
static LSR_ZEROPAGEX: &[MicroOpPair] = &zeropagex_rmw(MicroOp::OpLSR);
static ROL_ABSOLUTE: &[MicroOpPair] = &absolute_rmw(MicroOp::OpROL);
static ROL_ABSOLUTEX: &[MicroOpPair] = &absolutex_rmw(MicroOp::OpROL);
static ROL_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpROLImplied);
static ROL_ZEROPAGE: &[MicroOpPair] = &zeropage_rmw(MicroOp::OpROL);
static ROL_ZEROPAGEX: &[MicroOpPair] = &zeropagex_rmw(MicroOp::OpROL);
static ROR_ABSOLUTE: &[MicroOpPair] = &absolute_rmw(MicroOp::OpROR);
static ROR_ABSOLUTEX: &[MicroOpPair] = &absolutex_rmw(MicroOp::OpROR);
static ROR_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpRORImplied);
static ROR_ZEROPAGE: &[MicroOpPair] = &zeropage_rmw(MicroOp::OpROR);
static ROR_ZEROPAGEX: &[MicroOpPair] = &zeropagex_rmw(MicroOp::OpROR);

static JMP_ABSOLUTE: &[MicroOpPair] = &[
    MicroOpPair::from(MicroOp::FetchOpcode),
    MicroOpPair::from(MicroOp::FetchAdl),
    MicroOpPair::from(MicroOp::FetchAdh),
    MicroOpPair::pair(MicroOp::OpJMP, MicroOp::FetchOpcode),
];
static JMP_INDIRECT: &[MicroOpPair] = &indirect(MicroOp::OpJMP);
static JSR_ABSOLUTE: &[MicroOpPair] = &[
    MicroOpPair::from(MicroOp::FetchOpcode),
    MicroOpPair::from(MicroOp::FetchAdl),
    MicroOpPair::from(MicroOp::OpJSR),
    MicroOpPair::from(MicroOp::OpJSR),
    MicroOpPair::from(MicroOp::OpJSR),
    MicroOpPair::from(MicroOp::OpJSR),
    MicroOpPair::pair(MicroOp::OpJSR, MicroOp::FetchOpcode),
];

static RTS_IMPLIED: &[MicroOpPair] = &implied_6c(MicroOp::OpRTS);
static BRK_IMPLIED: &[MicroOpPair] = &[
    MicroOpPair::from(MicroOp::FetchOpcode),
    MicroOpPair::from(MicroOp::FetchAdl),
    MicroOpPair::from(MicroOp::OpBRK),
    MicroOpPair::from(MicroOp::OpBRK),
    MicroOpPair::from(MicroOp::OpBRK),
    MicroOpPair::from(MicroOp::OpBRK),
    MicroOpPair::from(MicroOp::OpBRK),
    MicroOpPair::from(MicroOp::FetchOpcode),
];
static RTI_IMPLIED: &[MicroOpPair] = &implied_6c(MicroOp::OpRTI);

static BCC_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBCC);
static BCS_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBCS);
static BEQ_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBEQ);
static BMI_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBMI);
static BNE_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBNE);
static BPL_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBPL);
static BVC_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBVC);
static BVS_RELATIVE: &[MicroOpPair] = &relative(MicroOp::OpBVS);
static CLC_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpCLC);
static CLD_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpCLD);
static CLI_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpCLI);
static CLV_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpCLV);
static SEC_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpSEC);
static SED_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpSED);
static SEI_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpSEI);
static NOP_IMPLIED: &[MicroOpPair] = &implied(MicroOp::OpNOP);

static ALR_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpALR);
static ANE_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpANE);
static ANX_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpANX);
static AXS_IMMEDIATE: &[MicroOpPair] = &immediate(MicroOp::OpAXS);
static LAX_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpLAX);
static LAX_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpLAX);
static LAX_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpLAX);
static LAX_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpLAX);
static LAX_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpLAX);
static LAX_ZEROPAGEY: &[MicroOpPair] = &zeropagey_read(MicroOp::OpLAX);
static LSE_ABSOLUTE: &[MicroOpPair] = &absolute_read(MicroOp::OpLSE);
static LSE_ABSOLUTEX: &[MicroOpPair] = &absolutex_read(MicroOp::OpLSE);
static LSE_ABSOLUTEY: &[MicroOpPair] = &absolutey_read(MicroOp::OpLSE);
static LSE_INDIRECTX: &[MicroOpPair] = &indirectx_read(MicroOp::OpLSE);
static LSE_INDIRECTY: &[MicroOpPair] = &indirecty_read(MicroOp::OpLSE);
static LSE_ZEROPAGE: &[MicroOpPair] = &zeropage_read(MicroOp::OpLSE);
static LSE_ZEROPAGEX: &[MicroOpPair] = &zeropagex_read(MicroOp::OpLSE);

pub fn decode_opcode(opcode: u8) -> &'static [MicroOpPair] {
    match opcode {
        0x00 => &BRK_IMPLIED,
        0x01 => &ORA_INDIRECTX,
        0x05 => &ORA_ZEROPAGE,
        0x06 => &ASL_ZEROPAGE,
        0x08 => &PHP_IMPLIED,
        0x09 => &ORA_IMMEDIATE,
        0x0a => &ASL_IMPLIED,
        0x0d => &ORA_ABSOLUTE,
        0x0e => &ASL_ABSOLUTE,
        0x10 => &BPL_RELATIVE,
        0x11 => &ORA_INDIRECTY,
        0x15 => &ORA_ZEROPAGEX,
        0x16 => &ASL_ZEROPAGEX,
        0x18 => &CLC_IMPLIED,
        0x19 => &ORA_ABSOLUTEY,
        0x1d => &ORA_ABSOLUTEX,
        0x1e => &ASL_ABSOLUTEX,
        0x20 => &JSR_ABSOLUTE,
        0x21 => &AND_INDIRECTX,
        0x24 => &BIT_ZEROPAGE,
        0x25 => &AND_ZEROPAGE,
        0x26 => &ROL_ZEROPAGE,
        0x28 => &PLP_IMPLIED,
        0x29 => &AND_IMMEDIATE,
        0x2a => &ROL_IMPLIED,
        0x2c => &BIT_ABSOLUTE,
        0x2d => &AND_ABSOLUTE,
        0x2e => &ROL_ABSOLUTE,
        0x30 => &BMI_RELATIVE,
        0x31 => &AND_INDIRECTY,
        0x35 => &AND_ZEROPAGEX,
        0x36 => &ROL_ZEROPAGEX,
        0x38 => &SEC_IMPLIED,
        0x39 => &AND_ABSOLUTEY,
        0x3a => &NOP_IMPLIED,
        0x3d => &AND_ABSOLUTEX,
        0x3e => &ROL_ABSOLUTEX,
        0x40 => &RTI_IMPLIED,
        0x41 => &EOR_INDIRECTX,
        0x43 => &LSE_INDIRECTX,
        0x45 => &EOR_ZEROPAGE,
        0x46 => &LSR_ZEROPAGE,
        0x47 => &LSE_ZEROPAGE,
        0x48 => &PHA_IMPLIED,
        0x49 => &EOR_IMMEDIATE,
        0x4a => &LSR_IMPLIED,
        0x4b => &ALR_IMMEDIATE,
        0x4c => &JMP_ABSOLUTE,
        0x4d => &EOR_ABSOLUTE,
        0x4e => &LSR_ABSOLUTE,
        0x4f => &LSE_ABSOLUTE,
        0x50 => &BVC_RELATIVE,
        0x51 => &EOR_INDIRECTY,
        0x53 => &LSE_INDIRECTY,
        0x55 => &EOR_ZEROPAGEX,
        0x56 => &LSR_ZEROPAGEX,
        0x57 => &LSE_ZEROPAGEX,
        0x58 => &CLI_IMPLIED,
        0x59 => &EOR_ABSOLUTEY,
        0x5b => &LSE_ABSOLUTEY,
        0x5d => &EOR_ABSOLUTEX,
        0x5e => &LSR_ABSOLUTEX,
        0x5f => &LSE_ABSOLUTEX,
        0x60 => &RTS_IMPLIED,
        0x61 => &ADC_INDIRECTX,
        0x65 => &ADC_ZEROPAGE,
        0x66 => &ROR_ZEROPAGE,
        0x68 => &PLA_IMPLIED,
        0x69 => &ADC_IMMEDIATE,
        0x6a => &ROR_IMPLIED,
        0x6c => &JMP_INDIRECT,
        0x6d => &ADC_ABSOLUTE,
        0x6e => &ROR_ABSOLUTE,
        0x70 => &BVS_RELATIVE,
        0x71 => &ADC_INDIRECTY,
        0x75 => &ADC_ZEROPAGEX,
        0x76 => &ROR_ZEROPAGEX,
        0x78 => &SEI_IMPLIED,
        0x79 => &ADC_ABSOLUTEY,
        0x7d => &ADC_ABSOLUTEX,
        0x7e => &ROR_ABSOLUTEX,
        0x80 => &NOP_IMPLIED,
        0x81 => &STA_INDIRECTX,
        0x84 => &STY_ZEROPAGE,
        0x85 => &STA_ZEROPAGE,
        0x86 => &STX_ZEROPAGE,
        0x88 => &DEY_IMPLIED,
        0x8a => &TXA_IMPLIED,
        0x8b => &ANE_IMMEDIATE,
        0x8c => &STY_ABSOLUTE,
        0x8d => &STA_ABSOLUTE,
        0x8e => &STX_ABSOLUTE,
        0x90 => &BCC_RELATIVE,
        0x91 => &STA_INDIRECTY,
        0x94 => &STY_ZEROPAGEX,
        0x95 => &STA_ZEROPAGEX,
        0x96 => &STX_ZEROPAGEY,
        0x98 => &TYA_IMPLIED,
        0x99 => &STA_ABSOLUTEY,
        0x9a => &TXS_IMPLIED,
        0x9d => &STA_ABSOLUTEX,
        0xa0 => &LDY_IMMEDIATE,
        0xa1 => &LDA_INDIRECTX,
        0xa2 => &LDX_IMMEDIATE,
        0xa3 => &LAX_INDIRECTX,
        0xa4 => &LDY_ZEROPAGE,
        0xa5 => &LDA_ZEROPAGE,
        0xa6 => &LDX_ZEROPAGE,
        0xa7 => &LAX_ZEROPAGE,
        0xa8 => &TAY_IMPLIED,
        0xa9 => &LDA_IMMEDIATE,
        0xaa => &TAX_IMPLIED,
        0xab => &ANX_IMMEDIATE,
        0xac => &LDY_ABSOLUTE,
        0xad => &LDA_ABSOLUTE,
        0xae => &LDX_ABSOLUTE,
        0xaf => &LAX_ABSOLUTE,
        0xb0 => &BCS_RELATIVE,
        0xb1 => &LDA_INDIRECTY,
        0xb3 => &LAX_INDIRECTY,
        0xb4 => &LDY_ZEROPAGEX,
        0xb5 => &LDA_ZEROPAGEX,
        0xb6 => &LDX_ZEROPAGEY,
        0xb7 => &LAX_ZEROPAGEY,
        0xb8 => &CLV_IMPLIED,
        0xb9 => &LDA_ABSOLUTEY,
        0xba => &TSX_IMPLIED,
        0xbc => &LDY_ABSOLUTEX,
        0xbd => &LDA_ABSOLUTEX,
        0xbe => &LDX_ABSOLUTEY,
        0xbf => &LAX_ABSOLUTEY,
        0xc0 => &CPY_IMMEDIATE,
        0xc1 => &CMP_INDIRECTX,
        0xc4 => &CPY_ZEROPAGE,
        0xc5 => &CMP_ZEROPAGE,
        0xc6 => &DEC_ZEROPAGE,
        0xc8 => &INY_IMPLIED,
        0xc9 => &CMP_IMMEDIATE,
        0xca => &DEX_IMPLIED,
        0xcb => &AXS_IMMEDIATE,
        0xcc => &CPY_ABSOLUTE,
        0xcd => &CMP_ABSOLUTE,
        0xce => &DEC_ABSOLUTE,
        0xd0 => &BNE_RELATIVE,
        0xd1 => &CMP_INDIRECTY,
        0xd5 => &CMP_ZEROPAGEX,
        0xd6 => &DEC_ZEROPAGEX,
        0xd8 => &CLD_IMPLIED,
        0xd9 => &CMP_ABSOLUTEY,
        0xdd => &CMP_ABSOLUTEX,
        0xde => &DEC_ABSOLUTEX,
        0xe0 => &CPX_IMMEDIATE,
        0xe1 => &SBC_INDIRECTX,
        0xe4 => &CPX_ZEROPAGE,
        0xe5 => &SBC_ZEROPAGE,
        0xe6 => &INC_ZEROPAGE,
        0xe8 => &INX_IMPLIED,
        0xe9 => &SBC_IMMEDIATE,
        0xea => &NOP_IMPLIED,
        0xec => &CPX_ABSOLUTE,
        0xed => &SBC_ABSOLUTE,
        0xee => &INC_ABSOLUTE,
        0xf0 => &BEQ_RELATIVE,
        0xf1 => &SBC_INDIRECTY,
        0xf5 => &SBC_ZEROPAGEX,
        0xf6 => &INC_ZEROPAGEX,
        0xf8 => &SED_IMPLIED,
        0xf9 => &SBC_ABSOLUTEY,
        0xfc => &NOP_IMPLIED,
        0xfd => &SBC_ABSOLUTEX,
        0xfe => &INC_ABSOLUTEX,
        _ => panic!("invalid opcode 0x{:x} at 0x{:x}", opcode, 0),
    }
}

static IRQ: &[MicroOpPair] = &[
    MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
    MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
    MicroOpPair::from(MicroOp::OpIRQ),
    MicroOpPair::from(MicroOp::OpIRQ),
    MicroOpPair::from(MicroOp::OpIRQ),
    MicroOpPair::from(MicroOp::OpIRQ),
    MicroOpPair::from(MicroOp::OpIRQ),
    MicroOpPair::from(MicroOp::FetchOpcode),
];
static NMI: &[MicroOpPair] = &[
    MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
    MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
    MicroOpPair::from(MicroOp::OpNMI),
    MicroOpPair::from(MicroOp::OpNMI),
    MicroOpPair::from(MicroOp::OpNMI),
    MicroOpPair::from(MicroOp::OpNMI),
    MicroOpPair::from(MicroOp::OpNMI),
    MicroOpPair::from(MicroOp::FetchOpcode),
];
static RESET: &[MicroOpPair] = &[
    MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
    MicroOpPair::from(MicroOp::FetchOpcodeDiscard),
    MicroOpPair::from(MicroOp::OpRST),
    MicroOpPair::from(MicroOp::OpRST),
    MicroOpPair::from(MicroOp::OpRST),
    MicroOpPair::from(MicroOp::OpRST),
    MicroOpPair::from(MicroOp::FetchOpcode),
];
static START: &[MicroOpPair] = &[
    MicroOpPair::from(MicroOp::FetchOpcode)
];

#[derive(Clone, Copy)]
pub enum ProgramId {
    Start,
    Irq,
    Nmi,
    Reset,
}

pub fn load_program(id: ProgramId) -> &'static [MicroOpPair] {
    match id {
        ProgramId::Start => &START,
        ProgramId::Irq => &IRQ,
        ProgramId::Nmi => &NMI,
        ProgramId::Reset => &RESET,
    }
}

