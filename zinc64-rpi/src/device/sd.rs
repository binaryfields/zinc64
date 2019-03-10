// This file is part of zinc64.
// Copyright (c) 2016-2019 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

use core::cmp;
use core::ops::Deref;
use core::result;
use register::{
    mmio::{ReadOnly, ReadWrite},
    register_bitfields, Field,
};

use super::delay;
use super::gpio;

const ACMD41_ARG_HC: u32 = 0x51ff8000;
const ACMD41_CMD_COMPLETE: u32 = 0x80000000;
const ACMD41_VOLTAGE: u32 = 0x00ff8000;
const CMD_ERRORS_MASK: u32 = 0xfff9c004;
const CMD_RCA_MASK: u32 = 0xffff0000;
const FREQ_SETUP: u32 = 400000;
const FREQ_NORMAL: u32 = 25000000;
const INT_ERROR_MASK: u32 = 0x017e8000;
const OCR_CARD_CAPACITY: u32 = 1 << 30;
const SCR_SD_BUS_WIDTH_4: u64 = 0x00000400;
const SCR_SUPP_SET_BLKCNT: u64 = 0x02000000;
const SD_FREQ: u32 = 41666667;
const ST_APP_CMD: u32 = 0x00000020;

register_bitfields! { u32,
    Control0 [
        HCTL_DWITDH OFFSET(1) NUMBITS(1) [],
        HCTL_HS_EN OFFSET(2) NUMBITS(1) [],
        SPI_MODE_EN OFFSET(20) NUMBITS(1) []
    ],
    Control1 [
        CLK_INTLEN OFFSET(0) NUMBITS(1) [],
        CLK_STABLE OFFSET(1) NUMBITS(1) [],
        CLK_EN OFFSET(2) NUMBITS(1) [],
        CLK_GENSEL OFFSET(5) NUMBITS(1) [],
        CLK_FREQ_MS2 OFFSET(6) NUMBITS(2) [],
        CLK_FREQ8 OFFSET(8) NUMBITS(8) [],
        DATA_TOUNIT OFFSET(16) NUMBITS(4) [
            Disabled = 0b1111,
            Max = 0b1110
        ],
        SRST_HC OFFSET(24) NUMBITS(1) [],
        SRST_CMD OFFSET(25) NUMBITS(1) [],
        SRST_DATA OFFSET(26) NUMBITS(1) []
    ],
    Interrupt [
        CMD_DONE OFFSET(0) NUMBITS(1) [],
        READ_RDY OFFSET(5) NUMBITS(1) [],
        CMD_TIMEOUT OFFSET(16) NUMBITS(1) [],
        DATA_TIMEOUT OFFSET(20) NUMBITS(1) []
    ],
    SlotIsr [
        SDVERSION OFFSET(16) NUMBITS(8) [
            V1 = 0,
            V2 = 1,
            V3 = 2
        ]
    ],
    Status [
        CMD_INHIBIT OFFSET(0) NUMBITS(1) [],
        DAT_INHIBIT OFFSET(1) NUMBITS(1) [],
        APP_CMD OFFSET(5) NUMBITS(1) [],
        READ_TRANSFER OFFSET(11) NUMBITS(1) []
    ]
}

#[repr(C)]
pub struct Registers {
    arg2: ReadWrite<u32>,
    blksizecnt: ReadWrite<u32>,
    arg1: ReadWrite<u32>,
    cmdtm: ReadWrite<u32>,
    resp0: ReadOnly<u32>,
    resp1: ReadOnly<u32>,
    resp2: ReadOnly<u32>,
    resp3: ReadOnly<u32>,
    data: ReadWrite<u32>,
    status: ReadWrite<u32, Status::Register>,
    control0: ReadWrite<u32, Control0::Register>,
    control1: ReadWrite<u32, Control1::Register>,
    interrupt: ReadWrite<u32, Interrupt::Register>,
    int_mask: ReadWrite<u32>,
    int_en: ReadWrite<u32>,
    control2: ReadWrite<u32>,
    _reserved_0: [u32; 9],
    slotisr_ver: ReadWrite<u32, SlotIsr::Register>,
}

enum CardType {
    Unknown,
    Type2Sc,
    Type2Hc,
}

#[derive(Clone, Copy)]
enum Command {
    GoIdle = 0x00000000,
    AllSendCid = 0x02010000,
    SendRelAddr = 0x03020000,
    CardSelect = 0x07030000,
    SendIfCond = 0x08020000,
    StopTrans = 0x0C030000,
    ReadSingle = 0x11220010,
    ReadMulti = 0x12220032,
    SetBlockcnt = 0x17020000,
    AppCmd = 0x37000000,
    AppCmdRspns48 = 0x37020000,
    SetBusWidth = 0x06020000,
    SendOpCond = 0x29020000,
    SendScr = 0x33220010,
}

impl Command {
    pub fn delay(&self) -> Option<u32> {
        match self {
            Command::AppCmd => Some(100),
            Command::SendIfCond => Some(100),
            Command::SendOpCond => Some(1000),
            _ => None,
        }
    }

    pub fn needs_app_cmd(&self) -> bool {
        match self {
            Command::SendOpCond => true,
            Command::SendScr => true,
            Command::SetBusWidth => true,
            _ => false,
        }
    }

    pub fn response_type(&self) -> ResponseType {
        match self {
            Command::GoIdle => ResponseType::None,
            Command::AppCmd => ResponseType::None,
            Command::CardSelect => ResponseType::Resp48BitWithBusy,
            Command::StopTrans => ResponseType::Resp48BitWithBusy,
            Command::AllSendCid => ResponseType::Resp136Bit,
            _ => ResponseType::Resp48Bit,
        }
    }

    pub fn val(&self) -> u32 {
        *self as u32
    }
}

pub enum Error {
    CmdError,
    Interrupt,
    InvalidResponse,
    InvalidVoltage,
    SendIfCondError,
    Timeout,
}

enum Response {
    None,
    Resp48(u32),
    Resp136([u32; 4]),
}

impl Response {
    pub fn to_resp48(&self) -> Result<u32> {
        match self {
            Response::Resp48(value) => Ok(*value),
            _ => Err(Error::InvalidResponse),
        }
    }
}

enum ResponseType {
    None = 0b00,
    Resp136Bit = 0b01,
    Resp48Bit = 0b10,
    Resp48BitWithBusy = 0b11,
}

pub type Result<T> = result::Result<T, Error>;

pub struct Sd {
    base_addr: usize,
    card_type: CardType,
    scr: u64,
    rca: u32,
}

impl Sd {
    pub fn new(base_addr: usize) -> Sd {
        Sd {
            base_addr,
            card_type: CardType::Unknown,
            scr: 0,
            rca: 0,
        }
    }

    pub fn init(&mut self, gpio: &gpio::GPIO) -> Result<()> {
        info!("Initializing GPIO ...");
        self.init_gpio(gpio);
        info!("Resetting controller ...");
        self.reset()?;
        info!("Checking voltage ...");
        self.send_if_cond(0x000001aa)?;
        let ocr = self.send_op_cond(ACMD41_ARG_HC)?;
        if ocr & ACMD41_VOLTAGE == 0 {
            return Err(Error::InvalidVoltage);
        }
        self.card_type = if ocr & OCR_CARD_CAPACITY != 0 {
            CardType::Type2Hc
        } else {
            CardType::Type2Sc
        };
        info!("Initializing controller ...");
        self.send_cmd(&Command::AllSendCid, 0)?;
        self.rca = self.send_cmd(&Command::SendRelAddr, 0)?.to_resp48()? & CMD_RCA_MASK;
        self.clk(FREQ_NORMAL)?;
        self.send_cmd(&Command::CardSelect, self.rca)?;
        self.scr = self.read_scr()?;
        if self.scr & SCR_SD_BUS_WIDTH_4 != 0 {
            self.send_cmd(&Command::SetBusWidth, self.rca | 2)?;
            self.control0.modify(Control0::HCTL_DWITDH::SET);
        }
        info!("Controller initialized.");
        Ok(())
    }

    pub fn read(&self, lba: u32, num: u32, buffer: &mut [u8]) -> result::Result<(), &'static str> {
        self.read_block(lba, num, buffer)
            .map_err(|_| "failed to read block")
    }

    fn init_gpio(&self, gpio: &gpio::GPIO) {
        // gpio_cd
        gpio.GPFSEL4.set(gpio.GPFSEL4.get() & (!(7 << (7 * 3))));
        gpio.GPPUD.write(gpio::GPPUD::PUD::Reserved);
        delay::wait_cycles(150);
        gpio.GPPUDCLK1.set(1 << 15);
        delay::wait_cycles(150);
        gpio.GPPUD.write(gpio::GPPUD::PUD::Off);
        gpio.GPPUDCLK1.set(0);
        gpio.GPHEN1.set(gpio.GPHEN1.get() | (1 << 15));
        // gpio_clk, gpio_cd
        gpio.GPFSEL4
            .set(gpio.GPFSEL4.get() | (7 << (8 * 3)) | (7 << (9 * 3)));
        gpio.GPPUD.write(gpio::GPPUD::PUD::Reserved);
        delay::wait_cycles(150);
        gpio.GPPUDCLK1.set((1 << 16) | (1 << 17));
        delay::wait_cycles(150);
        gpio.GPPUD.write(gpio::GPPUD::PUD::Off);
        gpio.GPPUDCLK1.set(0);
        // gpio_dat0/1/2/3
        gpio.GPFSEL5.set(
            gpio.GPFSEL5.get()
                | ((7 << (0 * 3)) | (7 << (1 * 3)) | (7 << (2 * 3)) | (7 << (3 * 3))),
        );
        gpio.GPPUD.write(gpio::GPPUD::PUD::Reserved);
        delay::wait_cycles(150);
        gpio.GPPUDCLK1
            .set((1 << 18) | (1 << 19) | (1 << 20) | (1 << 21));
        delay::wait_cycles(150);
        gpio.GPPUD.write(gpio::GPPUD::PUD::Off);
        gpio.GPPUDCLK1.set(0);
    }

    fn clk(&self, freq: u32) -> Result<()> {
        let mut divider = cmp::min((SD_FREQ + freq - 1) / freq, 0x3ff);
        if self.slotisr_ver.read(SlotIsr::SDVERSION) < 2 {
            let mut shift = find_last_set_bit(divider);
            if shift > 0 {
                shift -= 1;
            }
            if shift > 7 {
                shift = 7;
            }
            divider = 1 << shift;
        } else {
            if divider < 3 {
                divider = 4;
            }
        }
        debug!("Clock divider {}, freq {}", divider, SD_FREQ / divider);
        self.wait_if_busy()?;
        self.control1.modify(Control1::CLK_EN::CLEAR);
        delay::wait_msec(10);
        self.control1
            .modify(Control1::CLK_FREQ8.val(divider & 0xff));
        self.control1
            .modify(Control1::CLK_FREQ_MS2.val((divider & 0x300) >> 8));
        delay::wait_msec(10);
        self.control1.modify(Control1::CLK_EN::SET);
        delay::wait_msec(10);
        let mut counter = 10000;
        while !self.control1.is_set(Control1::CLK_STABLE) && counter != 0 {
            delay::wait_msec(10);
            counter -= 1;
        }
        if counter == 0 {
            Err(Error::Timeout)
        } else {
            Ok(())
        }
    }

    fn send_cmd(&self, command: &Command, arg: u32) -> Result<Response> {
        let issue_app_cmd = command.needs_app_cmd();
        if issue_app_cmd {
            self.send_app_cmd()?;
        }
        let response = self.send_cmd_int(command, arg)?;
        if issue_app_cmd && self.rca != 0 {
            if response.to_resp48()? & ST_APP_CMD == 0 {
                return Err(Error::CmdError);
            }
        }
        Ok(response)
    }

    fn send_cmd_int(&self, command: &Command, arg: u32) -> Result<Response> {
        debug!("Issuing command 0x{:08x} arg 0x{:08x}", command.val(), arg);
        self.wait_to_clear(Status::CMD_INHIBIT)?;
        self.interrupt.set(self.interrupt.get());
        self.arg1.set(arg);
        self.cmdtm.set(command.val());
        if let Some(delay) = command.delay() {
            delay::wait_msec(delay);
        }
        self.wait_for_interrupt(Interrupt::CMD_DONE)?;
        match command.response_type() {
            // No response
            ResponseType::None => Ok(Response::None),
            ResponseType::Resp48BitWithBusy => {
                let resp = self.resp0.get();
                if resp & CMD_ERRORS_MASK == 0 {
                    Ok(Response::Resp48(resp))
                } else {
                    Err(Error::CmdError)
                }
            }
            // RESP0 contains card status
            ResponseType::Resp48Bit => {
                let resp = self.resp0.get();
                let status = match command {
                    // RESP0 contains RCA and status bits 23,22,19,12:0
                    Command::SendRelAddr => {
                        (resp & 0x00001fff) | // 12:0 map directly to status 12:0
                            ((resp & 0x00002000) << 6) | // 13 maps to status 19 ERROR
                            ((resp & 0x00004000) << 8) | // 14 maps to status 22 ILLEGAL_COMMAND
                            ((resp & 0x00008000) << 8)
                    }
                    // RESP0 should match arg
                    Command::SendIfCond => 0,
                    // RESP0 contains OCR register
                    Command::SendOpCond => 0,
                    // RESP0 contains card status
                    _ => resp,
                };
                if status & CMD_ERRORS_MASK == 0 {
                    Ok(Response::Resp48(resp))
                } else {
                    Err(Error::CmdError)
                }
            }
            // RESP0..3 contains 128 bit CID or CSD shifted down by 8 bits as no CRC
            ResponseType::Resp136Bit => {
                let mut data = [0u32; 4];
                data[3] = self.resp0.get();
                data[2] = self.resp1.get();
                data[1] = self.resp2.get();
                data[0] = self.resp3.get();
                Ok(Response::Resp136(data))
            }
        }
    }

    fn send_app_cmd(&self) -> Result<()> {
        if self.rca == 0 {
            self.send_cmd_int(&Command::AppCmd, 0)?;
            Ok(())
        } else {
            let status = self
                .send_cmd_int(&Command::AppCmdRspns48, self.rca)?
                .to_resp48()?;
            if status & ST_APP_CMD != 0 {
                Ok(())
            } else {
                Err(Error::CmdError)
            }
        }
    }

    fn send_if_cond(&self, arg: u32) -> Result<u32> {
        let resp = self.send_cmd(&Command::SendIfCond, arg)?.to_resp48()?;
        if resp == arg {
            Ok(resp)
        } else {
            Err(Error::SendIfCondError)
        }
    }

    fn send_op_cond(&self, arg: u32) -> Result<u32> {
        let mut counter = 6;
        while counter > 0 {
            let ocr = match self.send_cmd(&Command::SendOpCond, arg) {
                Ok(resp) => Ok(resp.to_resp48()?),
                Err(Error::Timeout) if counter > 0 => Ok(0),
                Err(err) => Err(err),
            }?;
            if ocr & ACMD41_CMD_COMPLETE != 0 {
                return Ok(ocr);
            }
            delay::wait_msec(400);
            counter -= 1;
        }
        Err(Error::Timeout)
    }

    fn read_block(&self, lba: u32, num: u32, buffer: &mut [u8]) -> Result<()> {
        debug!("Read block {} #{}", lba, num);
        self.wait_to_clear(Status::DAT_INHIBIT)?;
        if num > 1 && self.scr & SCR_SUPP_SET_BLKCNT != 0 {
            self.send_cmd(&Command::SetBlockcnt, num)?;
        }
        self.blksizecnt.set((num << 16) | 512);
        let read_command = if num == 1 {
            Command::ReadSingle
        } else {
            Command::ReadMulti
        };
        let address = match self.card_type {
            CardType::Type2Sc => lba << 9,
            _ => lba,
        };
        self.send_cmd(&read_command, address)?;
        for block in 0..num {
            self.wait_for_interrupt(Interrupt::READ_RDY)?;
            let offset = block as usize * 512;
            for i in (offset..offset + 512).step_by(4) {
                let data = self.data.get();
                buffer[i] = (data & 0xff) as u8;
                buffer[i + 1] = ((data >> 8) & 0xff) as u8;
                buffer[i + 2] = ((data >> 16) & 0xff) as u8;
                buffer[i + 3] = ((data >> 24) & 0xff) as u8;
            }
        }
        if num > 1 && self.scr & SCR_SUPP_SET_BLKCNT == 0 {
            self.send_cmd(&Command::StopTrans, 0)?;
        }
        Ok(())
    }

    fn read_scr(&self) -> Result<u64> {
        self.wait_to_clear(Status::DAT_INHIBIT)?;
        self.blksizecnt.set((1 << 16) | 8);
        self.send_cmd(&Command::SendScr, 0)?;
        self.wait_for_interrupt(Interrupt::READ_RDY)?;
        let mut count = 100000;
        let mut data = [0u32; 2];
        for out in data.iter_mut() {
            while !self.status.is_set(Status::READ_TRANSFER) && count != 0 {
                delay::wait_msec(1);
                count -= 1;
            }
            if count > 0 {
                *out = self.data.get();
            }
        }
        if count == 0 {
            Err(Error::Timeout)
        } else {
            Ok((data[0] as u64) | ((data[1] as u64) << 32))
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.control0.set(0);
        self.control1.set(0);
        self.control1.modify(Control1::SRST_HC::SET);
        let mut count = 10000;
        loop {
            delay::wait_msec(10);
            if !self.control1.is_set(Control1::SRST_HC) || count == 0 {
                break;
            }
            count -= 1;
        }
        if count == 0 {
            return Err(Error::Timeout);
        }
        self.control1.modify(Control1::CLK_INTLEN::SET);
        self.control1.modify(Control1::DATA_TOUNIT::Max);
        delay::wait_msec(10);
        self.clk(FREQ_SETUP)?;
        self.int_en.set(0xffff_ffff);
        self.int_mask.set(0xffff_ffff);
        self.send_cmd(&Command::GoIdle, 0)?;
        self.card_type = CardType::Unknown;
        self.scr = 0;
        self.rca = 0;
        Ok(())
    }

    fn wait_for_interrupt(&self, field: Field<u32, Interrupt::Register>) -> Result<()> {
        let mut counter = 1000000u32;
        while !self.interrupt.is_set(field)
            && self.interrupt.get() & INT_ERROR_MASK == 0
            && counter != 0
        {
            delay::wait_msec(1);
            counter -= 1;
        }
        if counter == 0
            || self.interrupt.is_set(Interrupt::CMD_TIMEOUT)
            || self.interrupt.is_set(Interrupt::DATA_TIMEOUT)
        {
            self.interrupt.set(self.interrupt.get());
            Err(Error::Timeout)
        } else if self.interrupt.get() & INT_ERROR_MASK != 0 {
            self.interrupt.set(self.interrupt.get());
            Err(Error::Interrupt)
        } else {
            self.interrupt.write(field.val(1));
            Ok(())
        }
    }

    fn wait_if_busy(&self) -> Result<()> {
        let mut counter = 100000u32;
        while (self.status.is_set(Status::CMD_INHIBIT) || self.status.is_set(Status::DAT_INHIBIT))
            && counter != 0
        {
            delay::wait_msec(1);
            counter -= 1;
        }
        if counter == 0 {
            Err(Error::Timeout)
        } else {
            Ok(())
        }
    }

    fn wait_to_clear(&self, field: Field<u32, Status::Register>) -> Result<()> {
        let mut counter = 500000u32;
        while self.status.is_set(field)
            && self.interrupt.get() & INT_ERROR_MASK == 0
            && counter != 0
        {
            delay::wait_msec(1);
            counter -= 1;
        }
        if counter == 0 {
            Err(Error::Timeout)
        } else if self.interrupt.get() & INT_ERROR_MASK != 0 {
            Err(Error::Interrupt)
        } else {
            Ok(())
        }
    }

    fn ptr(&self) -> *const Registers {
        self.base_addr as *const _
    }
}

impl Deref for Sd {
    type Target = Registers;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr() }
    }
}

fn find_last_set_bit(mut x: u32) -> u32 {
    let mut shift = 32;
    if x == 0 {
        shift = 0;
    } else {
        if x & 0xffff0000 == 0 {
            x <<= 16;
            shift -= 16;
        }
        if x & 0xff000000 == 0 {
            x <<= 8;
            shift -= 8;
        }
        if x & 0xf0000000 == 0 {
            x <<= 4;
            shift -= 4;
        }
        if x & 0xc0000000 == 0 {
            x <<= 2;
            shift -= 2;
        }
        if x & 0x80000000 == 0 {
            shift -= 1;
        }
    }
    shift
}
