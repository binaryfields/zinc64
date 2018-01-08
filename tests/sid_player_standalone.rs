extern crate zinc64;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use zinc64::core::{
    Chip,
    Cpu,
    IoPort,
    IrqLine,
    MemoryController,
    Ram,
    SoundBuffer,
    TickFn,
    SystemModel,
};
use zinc64::cpu::Cpu6510;
use zinc64::sound::Sid;
use zinc64::sound::sid;

struct SimpleMemory {
    mode: u8,
    ram: Rc<RefCell<Ram>>,
    sid: Rc<RefCell<Chip>>,
}

// SimpleMemory permanently maps device I/O into memory map.

impl SimpleMemory {
    pub fn new(ram: Rc<RefCell<Ram>>, sid: Rc<RefCell<Chip>>) -> Self {
        SimpleMemory {
            mode: 0,
            ram,
            sid,
        }
    }
}

impl MemoryController for SimpleMemory {
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
    let cpu_io_port = Rc::new(RefCell::new(IoPort::new(0x00, 0xff)));
    let cpu_irq = Rc::new(RefCell::new(IrqLine::new("irq")));
    let cpu_nmi = Rc::new(RefCell::new(IrqLine::new("nmi")));
    let sound_buffer = Arc::new(Mutex::new(SoundBuffer::new(4096)));

    // Setup chipset
    let sid = Rc::new(RefCell::new(
        Sid::new(model.sid_model, sound_buffer.clone())
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
        cpu_io_port,
        cpu_irq,
        cpu_nmi,
        mem,
    );

    // Load program
    ram.borrow_mut().load(&CODE.to_vec(), CODE_OFFSET);
    cpu.set_pc(CODE_OFFSET);

    // Run it
    let clock = Rc::new(Cell::new(0u64));
    let clock_clone = clock.clone();
    let tick_fn: TickFn = Box::new(move || {
        clock_clone.set(clock_clone.get().wrapping_add(1));
    });

    let mut frames = 50;
    let mut delta = 0i32;
    while frames > 0 {
        // Run frame
        delta += model.cycles_per_frame as i32;
        let mut cycles = 0;
        while delta > 0 {
            let prev_clk = clock.get();
            cpu.step(&tick_fn);
            let elapsed = (clock.get() - prev_clk) as u32;
            cycles += elapsed;
            delta -= elapsed as i32;
        }
        // Produce audio (roughly 20ms)
        sid.borrow_mut().clock_delta(cycles);
        // ... do something with sound_buffer ...

        frames -= 1;
    }
}