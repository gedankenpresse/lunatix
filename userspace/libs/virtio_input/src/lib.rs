#![no_std]

use core::{
    alloc::Layout,
    ptr::{addr_of, addr_of_mut},
};

use io::read::ByteReader;
use liblunatix::{
    prelude::{syscall_abi::MapFlags, CAddr, CapabilityVariant},
    println,
};
use little_endian::LE;
use virtio::{DescriptorFlags, DeviceId, VirtDevice, VirtDeviceMM, VirtQ, VirtQMsgBuf};

const VIRTIO_DEVICE: usize = 0x10006000;
const VIRTIO_DEVICE_LEN: usize = 0x1000;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Absinfo {
    min: LE<u32>,
    max: LE<u32>,
    fuzz: LE<u32>,
    flat: LE<u32>,
    res: LE<u32>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Devids {
    bustype: LE<u16>,
    vendor: LE<u16>,
    product: LE<u16>,
    version: LE<u16>,
}

#[repr(C)]
pub union InputInfo {
    string: [u8; 128],
    bitmap: [u8; 128],
    abs: Absinfo,
    ids: Devids,
}

#[repr(C)]
pub struct InputConfig {
    select: u8,
    subsel: u8,
    size: u8,
    _reserved: [u8; 5],
    info: InputInfo,
}

#[repr(u8)]
pub enum ConfigSelect {
    Unset = 0x0,
    IdName = 0x1,
    IdSerial = 0x2,
    IdDevids = 0x3,
    PropBits = 0x10,
    EvBits = 0x11,
    AbsInfo = 0x12,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Event {
    pub event_type: EventType,
    pub code: u16,
    pub value: u32,
}
#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum EventType {
    Syn = 0x00,
    Key = 0x01,
    Rel = 0x02,
    Abs = 0x03,
    Msc = 0x04,
    Sw = 0x05,
    Led = 0x11,
    Snd = 0x12,
    Rep = 0x14,
    Ff = 0x15,
    Pwr = 0x16,
    FfStatus = 0x17,
    Max = 0x1f,
}

pub unsafe fn read_input_config(config: *mut InputConfig) {
    addr_of_mut!((*config).select).write_volatile(ConfigSelect::IdName as u8);
    addr_of_mut!((*config).subsel).write_volatile(0);
    let size = addr_of!((*config).size).read_volatile();
    let name_buf = addr_of!((*config).info.string).read_volatile();
    let name = core::str::from_utf8(&name_buf[0..size as usize]).unwrap();
    println!("{}", name);

    addr_of_mut!((*config).select).write_volatile(ConfigSelect::IdSerial as u8);
    addr_of_mut!((*config).subsel).write_volatile(0);
    let size = addr_of!((*config).size).read_volatile();
    let serial_buf = addr_of!((*config).info.string).read_volatile();
    println!("{:?}", &serial_buf[0..size as usize]);

    addr_of_mut!((*config).select).write_volatile(ConfigSelect::IdDevids as u8);
    addr_of_mut!((*config).subsel).write_volatile(0);
    let ids = addr_of!((*config).info.ids).read_volatile();
    println!("{:0x?}", &ids);

    addr_of_mut!((*config).select).write_volatile(ConfigSelect::IdDevids as u8);
    addr_of_mut!((*config).subsel).write_volatile(0);
    let size = addr_of!((*config).size).read_volatile();
    let ids = addr_of!((*config).info.bitmap).read_volatile();
    let bitmap = &ids[0..size as usize];
    println!("{:0x?}", &bitmap);
}

/// Allocate two buffers from the memory capability that are used for storing the actual P9 messages
fn prepare_msg_bufs(mem: CAddr, vspace: CAddr, size: usize) -> VirtQMsgBuf {
    let buf_region = mmap::allocate_raw(Layout::array::<Event>(size).unwrap()).unwrap();
    assert!(buf_region.bytes < 4096);
    let page1 = caddr_alloc::alloc_caddr();
    liblunatix::ipc::mem::derive(mem, page1, CapabilityVariant::Page, None).unwrap();
    liblunatix::ipc::page::map_page(
        page1,
        vspace,
        mem,
        buf_region.start as usize,
        MapFlags::READ | MapFlags::WRITE,
    )
    .unwrap();

    VirtQMsgBuf {
        buf: unsafe { core::slice::from_raw_parts_mut(buf_region.start, buf_region.bytes) },
        page: page1,
        paddr: liblunatix::ipc::page::get_paddr(page1).unwrap(),
    }
}

#[allow(unused)]
pub struct InputDriver {
    device: VirtDevice<InputConfig>,
    event_q: VirtQ,
    status_q: VirtQ,
    event_used_ack: u16,
    irq: CAddr,
    noti: CAddr,
    event_buf: VirtQMsgBuf,
}

impl InputDriver {
    pub unsafe fn read_event(&mut self) -> Event {
        let idx_addr = addr_of!(*self.event_q.used.idx);
        while self.event_used_ack == idx_addr.read_volatile() {
            liblunatix::ipc::irq::irq_complete(self.irq).unwrap();
            liblunatix::syscalls::wait_on(self.noti).unwrap();
        }
        assert_ne!(self.event_used_ack, idx_addr.read_volatile());
        let used_idx = self.event_used_ack % self.event_q.descriptor_table.len() as u16;
        self.event_used_ack = self.event_used_ack.wrapping_add(1);
        let used_elem = addr_of!(self.event_q.used.ring[used_idx as usize]).read_volatile();
        let desc_idx = used_elem.id as usize;
        let desc = addr_of!(self.event_q.descriptor_table[desc_idx]).read_volatile();
        let offset = desc.address - self.event_buf.paddr as u64;
        let buf_idx = offset as usize / core::mem::size_of::<Event>();
        let buf = core::slice::from_raw_parts_mut(
            self.event_buf.buf.as_mut_ptr().cast::<Event>(),
            self.event_q.descriptor_table.len(),
        );
        let event = addr_of!(buf[buf_idx]).read_volatile();
        self.event_q.avail.insert_request(desc_idx as u16);
        return event;
    }
}

pub fn init_input_driver(
    mem: CAddr,
    vspace: CAddr,
    devmem: CAddr,
    irq_control: CAddr,
) -> InputDriver {
    println!("input driver:");
    liblunatix::ipc::devmem::devmem_map(devmem, mem, vspace, VIRTIO_DEVICE, VIRTIO_DEVICE_LEN)
        .unwrap();
    let driver = unsafe {
        let device: VirtDevice<InputConfig> = VirtDevice::at(VIRTIO_DEVICE as *mut VirtDeviceMM);
        assert_eq!(device.mm.device_id.read(), DeviceId::INPUT_DEVICE);

        let _ = read_input_config(device.config);

        let mut status = device.mm.init();
        status = device.mm.negotiate_features(status, 0 as u64);

        // setup an irq handler for the virtio device
        let irq_notif = caddr_alloc::alloc_caddr();
        liblunatix::ipc::mem::derive(mem, irq_notif, CapabilityVariant::Notification, None)
            .unwrap();
        let irq = caddr_alloc::alloc_caddr();
        liblunatix::ipc::irq_control::irq_control_claim(irq_control, 0x06, irq, irq_notif).unwrap();

        let mut event_q = virtio::queue_setup(device.mm, 0, mem, vspace).unwrap();
        let status_q = virtio::queue_setup(device.mm, 1, mem, vspace).unwrap();

        let event_buf = prepare_msg_bufs(mem, vspace, event_q.descriptor_table.len());
        device.mm.finish_setup(status);

        let buf = core::slice::from_raw_parts_mut(
            event_buf.buf.as_mut_ptr().cast::<Event>(),
            event_q.descriptor_table.len(),
        );
        let base_addr = buf.as_ptr() as usize;
        for event in buf.iter_mut() {
            let event_addr = event as *const _ as usize;
            let offset = event_addr - base_addr;
            let (i, desc) = event_q.get_free_descriptor().unwrap();
            desc.address = event_buf.paddr as u64 + offset as u64;
            desc.length = core::mem::size_of::<Event>() as u32;
            desc.flags = DescriptorFlags::WRITE as u16;
            desc.next = 0;
            event_q.avail.insert_request(i as u16);
        }
        device.mm.notify(0);

        InputDriver {
            device,
            event_q,
            status_q,
            event_buf,
            event_used_ack: 0,
            irq,
            noti: irq_notif,
        }
    };
    return driver;
}

pub struct VirtioByteReader {
    pub input: InputDriver,
    pub shift: bool,
}

impl VirtioByteReader {
    fn ascii_alpha(&self, c: u8) -> u8 {
        if self.shift {
            return (c as char).to_ascii_uppercase() as u8;
        } else {
            return c;
        }
    }

    fn ascii_num(&self, c: u8) -> u8 {
        return c;
    }
}

// https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h
impl ByteReader for VirtioByteReader {
    #[rustfmt::skip]
    fn read_byte(&mut self) -> Result<u8, ()> {
        loop {
            let event = unsafe { self.input.read_event() };
            match (self.shift, event) {
                (_, Event { event_type: EventType::Key, value: 1, code: 2 }) => return Ok(self.ascii_num(b'1')),
                (_, Event { event_type: EventType::Key, value: 1, code: 4 }) => return Ok(self.ascii_num(b'3')),
                (_, Event { event_type: EventType::Key, value: 1, code: 5 }) => return Ok(self.ascii_num(b'4')),
                (_, Event { event_type: EventType::Key, value: 1, code: 6 }) => return Ok(self.ascii_num(b'5')),
                (_, Event { event_type: EventType::Key, value: 1, code: 3 }) => return Ok(self.ascii_num(b'2')),
                (_, Event { event_type: EventType::Key, value: 1, code: 7 }) => return Ok(self.ascii_num(b'6')),
                (_, Event { event_type: EventType::Key, value: 1, code: 8 }) => return Ok(self.ascii_num(b'7')),
                (_, Event { event_type: EventType::Key, value: 1, code: 9 }) => return Ok(self.ascii_num(b'8')),
                (_, Event { event_type: EventType::Key, value: 1, code: 10 }) => return Ok(self.ascii_num(b'9')),
                (_, Event { event_type: EventType::Key, value: 1, code: 11 }) => return Ok(self.ascii_num(b'0')),

                (_, Event { event_type: EventType::Key, value: 1, code: 16 }) => return Ok(self.ascii_alpha(b'q')),
                (_, Event { event_type: EventType::Key, value: 1, code: 17 }) => return Ok(self.ascii_alpha(b'w')),
                (_, Event { event_type: EventType::Key, value: 1, code: 18 }) => return Ok(self.ascii_alpha(b'e')),
                (_, Event { event_type: EventType::Key, value: 1, code: 19 }) => return Ok(self.ascii_alpha(b'r')),
                (_, Event { event_type: EventType::Key, value: 1, code: 20 }) => return Ok(self.ascii_alpha(b't')),
                (_, Event { event_type: EventType::Key, value: 1, code: 21 }) => return Ok(self.ascii_alpha(b'z')),
                (_, Event { event_type: EventType::Key, value: 1, code: 22 }) => return Ok(self.ascii_alpha(b'u')),
                (_, Event { event_type: EventType::Key, value: 1, code: 23 }) => return Ok(self.ascii_alpha(b'i')),
                (_, Event { event_type: EventType::Key, value: 1, code: 24 }) => return Ok(self.ascii_alpha(b'o')),
                (_, Event { event_type: EventType::Key, value: 1, code: 25 }) => return Ok(self.ascii_alpha(b'p')),

                (_, Event { event_type: EventType::Key, value: 1, code: 28 }) => return Ok(b'\n'),

                (_, Event { event_type: EventType::Key, value: 1, code: 30 }) => return Ok(self.ascii_alpha(b'a')),
                (_, Event { event_type: EventType::Key, value: 1, code: 31 }) => return Ok(self.ascii_alpha(b's')),
                (_, Event { event_type: EventType::Key, value: 1, code: 32 }) => return Ok(self.ascii_alpha(b'd')),
                (_, Event { event_type: EventType::Key, value: 1, code: 33 }) => return Ok(self.ascii_alpha(b'f')),
                (_, Event { event_type: EventType::Key, value: 1, code: 34 }) => return Ok(self.ascii_alpha(b'g')),
                (_, Event { event_type: EventType::Key, value: 1, code: 35 }) => return Ok(self.ascii_alpha(b'h')),
                (_, Event { event_type: EventType::Key, value: 1, code: 36 }) => return Ok(self.ascii_alpha(b'j')),
                (_, Event { event_type: EventType::Key, value: 1, code: 37 }) => return Ok(self.ascii_alpha(b'k')),
                (_, Event { event_type: EventType::Key, value: 1, code: 38 }) => return Ok(self.ascii_alpha(b'l')),


                (_ , Event { event_type: EventType::Key, value: 1, code: 42 }) => self.shift = true,
                (_ , Event { event_type: EventType::Key, value: 0, code: 42 }) => self.shift = false,

                (_, Event { event_type: EventType::Key, value: 1, code: 44 }) => return Ok(self.ascii_alpha(b'y')),
                (_, Event { event_type: EventType::Key, value: 1, code: 45 }) => return Ok(self.ascii_alpha(b'x')),
                (_, Event { event_type: EventType::Key, value: 1, code: 46 }) => return Ok(self.ascii_alpha(b'c')),
                (_, Event { event_type: EventType::Key, value: 1, code: 47 }) => return Ok(self.ascii_alpha(b'v')),
                (_, Event { event_type: EventType::Key, value: 1, code: 48 }) => return Ok(self.ascii_alpha(b'b')),
                (_, Event { event_type: EventType::Key, value: 1, code: 49 }) => return Ok(self.ascii_alpha(b'n')),
                (_, Event { event_type: EventType::Key, value: 1, code: 50 }) => return Ok(self.ascii_alpha(b'm')),


                (true, Event { event_type: EventType::Key, value: 1, code: 53 }) => return Ok(self.ascii_alpha(b'_')),
                (false, Event { event_type: EventType::Key, value: 1, code: 53 }) => return Ok(self.ascii_alpha(b'-')),

                (_, Event { event_type: EventType::Key, value: 1, code: 57 }) => return Ok(b' '),
                _ => {
                    if event.value == 1 {
                        println!("{:?}", event);
                    }
                }
            }
        }
    }
}
