// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use cortex_a::{barrier, regs::*};
use register::register_bitfields;

// SPEC: AArch64 Reference Manual p. 2451

register_bitfields! {u64,
    STAGE1_DESCRIPTOR [
        /// Execute-never
        XN OFFSET(54) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// Output address
        LVL1_OUTPUT_ADDR_4KB    OFFSET(30) NUMBITS(18) [], // [47:30]
        LVL2_OUTPUT_ADDR_4KB    OFFSET(21) NUMBITS(27) [], // [47:21]
        NEXT_LVL_TABLE_ADDR_4KB OFFSET(12) NUMBITS(36) [], // [47:12]
        PAGE_OUTPUT_ADDR_4KB    OFFSET(12) NUMBITS(36) [], // [47:12]

        /// Access flag
        AF OFFSET(10) NUMBITS(1) [
            False = 0,
            True = 1
        ],

        /// Shareability field
        SH OFFSET(8) NUMBITS(2) [
            NonShareable = 0b00,
            OuterShareable = 0b10,
            InnerShareable = 0b11
        ],

        /// Access Permissions
        AP OFFSET(6) NUMBITS(2) [
            RW_EL1 = 0b00,
            RW_EL1_EL0 = 0b01,
            RO_EL1 = 0b10,
            RO_EL1_EL0 = 0b11
        ],

        /// Memory attributes index into the MAIR_EL1 register
        AttrIndx OFFSET(2) NUMBITS(3) [],

        /// Descriptor type
        TYPE OFFSET(1) NUMBITS(1) [
            Block = 0,
            Table = 1
        ],

        VALID OFFSET(0) NUMBITS(1) [
            False = 0,
            True = 1
        ]
    ]
}

trait BaseAddr {
    fn base_addr(&self) -> u64;
}

impl BaseAddr for [u64; 512] {
    fn base_addr(&self) -> u64 {
        self as *const u64 as u64
    }
}

const NUM_ENTRIES_4KB: usize = 512;

#[repr(C)]
#[repr(align(4096))]
struct PageTable {
    entries: [u64; NUM_ENTRIES_4KB],
}

static mut LVL2_TABLE: PageTable = PageTable {
    entries: [0; NUM_ENTRIES_4KB],
};

#[allow(unused)]
static mut LVL3_TABLE_BLOCK_0: PageTable = PageTable {
    entries: [0; NUM_ENTRIES_4KB],
};

mod mair {
    pub const DEVICE: u64 = 0;
    pub const NORMAL: u64 = 1;
    pub const NORMAL_NON_CACHEABLE: u64 = 2;
}

pub unsafe fn init_page_table() {
    // SPEC: p. 2428 Table D5-18 Properties of the address lookup levels, 4KB granule size
    // Input address range is 30bit covering 1GB
    // Level 2 table, descriptor indexing address[29:21], size of addressed range 2MB (2^21)
    // 880MB ARM RAM
    LVL2_TABLE.entries[0] = (STAGE1_DESCRIPTOR::VALID::True
        + STAGE1_DESCRIPTOR::TYPE::Block
        + STAGE1_DESCRIPTOR::AttrIndx.val(mair::NORMAL)
        + STAGE1_DESCRIPTOR::AP::RW_EL1
        + STAGE1_DESCRIPTOR::SH::InnerShareable
        + STAGE1_DESCRIPTOR::AF::True
        + STAGE1_DESCRIPTOR::XN::False
        + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KB.val(0)).value;
    let ram_range = &mut LVL2_TABLE.entries[1..439];
    for (i, entry) in ram_range.iter_mut().enumerate() {
        *entry = (STAGE1_DESCRIPTOR::VALID::True
            + STAGE1_DESCRIPTOR::TYPE::Block
            + STAGE1_DESCRIPTOR::AttrIndx.val(mair::NORMAL)
            + STAGE1_DESCRIPTOR::AP::RW_EL1
            + STAGE1_DESCRIPTOR::SH::InnerShareable
            + STAGE1_DESCRIPTOR::AF::True
            + STAGE1_DESCRIPTOR::XN::True
            + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KB.val(1 + i as u64)).value;
    }
    // 128MB VC
    let vc_range = &mut LVL2_TABLE.entries[440..512 - 8];
    for (i, entry) in vc_range.iter_mut().enumerate() {
        *entry = (STAGE1_DESCRIPTOR::VALID::True
            + STAGE1_DESCRIPTOR::TYPE::Block
            + STAGE1_DESCRIPTOR::AttrIndx.val(mair::NORMAL_NON_CACHEABLE)
            + STAGE1_DESCRIPTOR::AP::RW_EL1
            + STAGE1_DESCRIPTOR::SH::OuterShareable
            + STAGE1_DESCRIPTOR::AF::True
            + STAGE1_DESCRIPTOR::XN::True
            + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KB.val(440 + i as u64)).value;
    }
    // 16MB peripherals
    let device_range = &mut LVL2_TABLE.entries[512 - 8..512];
    for (i, entry) in device_range.iter_mut().enumerate() {
        *entry = (STAGE1_DESCRIPTOR::VALID::True
            + STAGE1_DESCRIPTOR::TYPE::Block
            + STAGE1_DESCRIPTOR::AttrIndx.val(mair::DEVICE)
            + STAGE1_DESCRIPTOR::AP::RW_EL1
            + STAGE1_DESCRIPTOR::SH::OuterShareable
            + STAGE1_DESCRIPTOR::AF::True
            + STAGE1_DESCRIPTOR::XN::True
            + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KB.val(512 - 8 + i as u64)).value;
    }
    // 2MB mailboxes
    LVL2_TABLE.entries[439] = (STAGE1_DESCRIPTOR::VALID::True
        + STAGE1_DESCRIPTOR::TYPE::Block
        + STAGE1_DESCRIPTOR::AttrIndx.val(mair::DEVICE)
        + STAGE1_DESCRIPTOR::AP::RW_EL1
        + STAGE1_DESCRIPTOR::SH::OuterShareable
        + STAGE1_DESCRIPTOR::AF::True
        + STAGE1_DESCRIPTOR::XN::True
        + STAGE1_DESCRIPTOR::LVL2_OUTPUT_ADDR_4KB.val(439)).value;
}

pub unsafe fn init() {
    // Set memory attributes
    MAIR_EL1.write(
        MAIR_EL1::Attr2_HIGH::Memory_OuterNonCacheable
            + MAIR_EL1::Attr2_LOW_MEMORY::InnerNonCacheable
            + MAIR_EL1::Attr1_HIGH::Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc
            + MAIR_EL1::Attr1_LOW_MEMORY::InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc
            + MAIR_EL1::Attr0_HIGH::Device
            + MAIR_EL1::Attr0_LOW_DEVICE::Device_nGnRE,
    );

    // Point to level 2 translation table
    TTBR0_EL1.set_baddr(LVL2_TABLE.entries.base_addr());

    // Force all previous changes
    barrier::isb(barrier::SY);

    // Configure stage 1 translation
    let ips = ID_AA64MMFR0_EL1.read(ID_AA64MMFR0_EL1::PARange);
    TCR_EL1.write(
        TCR_EL1::TBI0::Ignored // TBD
            + TCR_EL1::IPS.val(ips)
            + TCR_EL1::TG0::KiB_4
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::T0SZ.val(34),
    );

    // Switch MMU on and enable page translation
    SCTLR_EL1.modify(
        SCTLR_EL1::M::Enable
            + SCTLR_EL1::C::Cacheable
            + SCTLR_EL1::I::Cacheable,
    );

    // Force MMU init to complete
    barrier::isb(barrier::SY);
}
