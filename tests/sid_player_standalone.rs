// This file is part of zinc64.
// Copyright (c) 2016-2018 Sebastian Jastrzebski. All rights reserved.
// Licensed under the GPLv3. See LICENSE file in the project root for full license text.

extern crate zinc64;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use zinc64::core::{
    Chip,
    Clock,
    Cpu,
    IoPort,
    IrqLine,
    Mmu,
    Pin,
    Ram,
    TickFn,
    SystemModel,
};
use zinc64::cpu::Cpu6510;
use zinc64::sound::Sid;
use zinc64::sound::sid;
use zinc64::system::CircularBuffer;

struct SimpleMemory {
    mode: u8,
    ram: Rc<RefCell<Ram>>,
    sid: Rc<RefCell<dyn Chip>>,
}

// SimpleMemory permanently maps device I/O into memory map.

impl SimpleMemory {
    pub fn new(ram: Rc<RefCell<Ram>>, sid: Rc<RefCell<dyn Chip>>) -> Self {
        SimpleMemory {
            mode: 0,
            ram,
            sid,
        }
    }
}

impl Mmu for SimpleMemory {
    fn switch_banks(&mut self, mode: u8) {
        self.mode = mode;
    }

    fn read(&self, address: u16) -> u8 {
        match address {
            0xd400 ... 0xd7ff => self.sid.borrow_mut().read((address & 0x001f) as u8),
            _ => self.ram.borrow().read(address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xd400 ... 0xd7ff => self.sid.borrow_mut().write((address & 0x001f) as u8, value),
            _ => self.ram.borrow_mut().write(address, value),
        }
    }
}

// NOTE this should be actual player code

static CODE: [u8; 24] = [
    0x78u8, 0xa9, 0xff, 0x8d, 0x02, 0xdc, 0xa9, 0x00, 0x8d, 0x03, 0xdc, 0xa9, 0xfd, 0x8d,
    0x00, 0xdc, 0xad, 0x01, 0xdc, 0x29, 0x20, 0xd0, 0xf9, 0x58,
];
static CODE_OFFSET: u16 = 0x1000;

#[test]
fn exec_sid_player() {
    let model = SystemModel::c64_pal();
    let clock = Rc::new(Clock::new());
    let ba_line = Rc::new(RefCell::new(Pin::new_high()));
    let cpu_io_port = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
    let irq_line = Rc::new(RefCell::new(IrqLine::new("irq")));
    let nmi_line = Rc::new(RefCell::new(IrqLine::new("nmi")));
    let sound_buffer = Arc::new(Mutex::new(CircularBuffer::new(4096)));

    // Setup chipset
    let sid = Rc::new(RefCell::new(
        Sid::new(model.sid_model, clock.clone(), sound_buffer.clone())
    ));
    sid.borrow_mut().set_sampling_parameters(
        sid::SamplingMethod::ResampleFast,
        model.cpu_freq,
        44100,
    );
    let ram = Rc::new(RefCell::new(
        Ram::new(model.memory_size)
    ));
    let mem = Rc::new(RefCell::new(
        SimpleMemory::new(ram.clone(), sid.clone())
    ));
    let mut cpu = Cpu6510::new(
        ba_line,
        cpu_io_port,
        irq_line,
        nmi_line,
        mem,
    );

    // Load program
    ram.borrow_mut().load(&CODE.to_vec(), CODE_OFFSET);
    cpu.set_pc(CODE_OFFSET);

    // Run it
    let clock_clone = clock.clone();
    let tick_fn: TickFn = Box::new(move || {
        clock_clone.tick();
    });

    let mut frames = 50;
    let mut delta = 0i32;
    while frames > 0 {
        // Run frame
        delta += model.cycles_per_frame as i32;
        while delta > 0 {
            let prev_clock = clock.get();
            cpu.step(&tick_fn);
            delta -= clock.elapsed(prev_clock) as i32
        }
        // Produce audio (roughly 20ms)
        sid.borrow_mut().process_vsync();
        // ... do something with sound_buffer ...

        frames -= 1;
    }
}