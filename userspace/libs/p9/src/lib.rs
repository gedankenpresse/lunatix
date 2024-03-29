#![no_std]

use bitflags::bitflags;
use core::fmt::{Debug, Formatter};
use core::mem;

/// Maximum number of walk elements in a single message
const MAXWELEM: usize = 16;

macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<u8> for $name {
            type Error = ();

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u8 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(u8)]
    pub enum P9MsgType {
        TVersion = 100,
        RVersion = 101,
        TAuth = 102,
        RAuth = 103,
        TAttach = 104,
        RAttach = 105,
        TError = 106,
        RError = 107,
        TFlush = 108,
        RFlush = 109,
        TWalk = 110,
        RWalk = 111,
        TOpen = 112,
        ROpen = 113,
        TCreate = 114,
        RCreate = 115,
        TRead = 116,
        RRead = 117,
        TWrite = 118,
        RWrite = 119,
        TClunk = 120,
        RClunk = 121,
        TRemove = 122,
        RRemove = 123,
        TStat = 124,
        RStat = 125,
        TWStat = 126,
        RWStat = 127,
        TMax = 128,
    }
}

back_to_enum! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    #[repr(u8)]
    pub enum P9QidType {
        File = 0,
        Link = 1,
        Symlink = 1 << 1,
        Tmp = 1 << 2,
        Auth = 1 << 3,
        Mount = 1 << 4,
        Excl = 1 << 5,
        Append = 1 << 6,
        Dir = 1 << 7,
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct P9Qid {
    pub typ: P9QidType,
    pub version: u32,
    pub path: u64,
}

impl P9Qid {
    fn deserialize(reader: &mut ByteReader) -> Option<Self> {
        let typ = reader.read_u8()?;
        let typ = P9QidType::try_from(typ).unwrap();
        let version = reader.read_u32()?;
        let path = reader.read_u64()?;
        Some(Self { typ, version, path })
    }
}

#[derive(Debug)]
pub struct P9RequestBuilder<'buf> {
    buf: &'buf mut [u8],
    fill_marker: usize,
}

impl<'buf> P9RequestBuilder<'buf> {
    pub fn new(buf: &'buf mut [u8]) -> Self {
        Self {
            buf,
            fill_marker: 4,
        }
    }

    pub fn write_type(&mut self, typ: P9MsgType) -> &mut Self {
        self.write_u8(typ as u8)
    }

    pub fn write_u8(&mut self, value: u8) -> &mut Self {
        self.buf[self.fill_marker] = value;
        self.fill_marker += 1;
        self
    }

    pub fn write_u16(&mut self, value: u16) -> &mut Self {
        self.buf[self.fill_marker..self.fill_marker + mem::size_of::<u16>()]
            .copy_from_slice(&value.to_le_bytes());
        self.fill_marker += mem::size_of::<u16>();
        self
    }

    pub fn write_u32(&mut self, value: u32) -> &mut Self {
        self.buf[self.fill_marker..self.fill_marker + mem::size_of::<u32>()]
            .copy_from_slice(&value.to_le_bytes());
        self.fill_marker += mem::size_of::<u32>();
        self
    }

    pub fn write_u64(&mut self, value: u64) -> &mut Self {
        self.buf[self.fill_marker..self.fill_marker + mem::size_of::<u64>()]
            .copy_from_slice(&value.to_le_bytes());
        self.fill_marker += mem::size_of::<u64>();
        self
    }

    pub fn write_str(&mut self, value: &str) -> &mut Self {
        self.write_u16(value.len() as u16);
        self.buf[self.fill_marker..self.fill_marker + value.len()]
            .copy_from_slice(value.as_bytes());
        self.fill_marker += value.len();
        self
    }

    pub fn finish(&mut self) {
        self.write_u32(self.fill_marker as u32 - 4);
    }
}

#[derive(Debug)]
pub struct ByteReader<'buf> {
    buf: &'buf [u8],
}

impl<'a> ByteReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf }
    }
    pub fn read_u8(&mut self) -> Option<u8> {
        let val = self.buf.get(0).copied()?;
        self.buf = &self.buf[1..];
        return Some(val);
    }
    pub fn read_u16(&mut self) -> Option<u16> {
        let a = self.read_slice(mem::size_of::<u16>())?;
        let v = u16::from_le_bytes(a.try_into().unwrap());
        return Some(v);
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        let a = self.read_slice(mem::size_of::<u32>())?;
        let v = u32::from_le_bytes(a.try_into().unwrap());
        return Some(v);
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        let a = self.read_slice(mem::size_of::<u64>())?;
        let v = u64::from_le_bytes(a.try_into().unwrap());
        return Some(v);
    }

    pub fn read_slice(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.buf.len() < len {
            return None;
        }
        let (a, rest) = self.buf.split_at(len);
        self.buf = rest;
        return Some(a);
    }

    pub fn read_str(&mut self) -> Option<&'a str> {
        let len = self.read_u16()? as usize;
        let s = self.read_slice(len)?;
        Some(core::str::from_utf8(s).unwrap())
    }

    pub fn remaining_data(self) -> &'a [u8] {
        self.buf
    }
}

#[derive(Debug)]
pub enum Request<'a> {
    Version(TVersion<'a>),
    Attach(TAttach<'a>),
    Walk(TWalk<'a>),
    Open(TOpen),
    Read(TRead),
    Clunk(TClunk),
}

#[derive(Debug)]
pub enum Response<'a> {
    Error(RError<'a>),
    Version(RVersion<'a>),
    Attach(RAttach),
    Open(ROpen),
    Flush(RFlush),
    // Create(RCreate<'a>),
    Read(RRead<'a>),
    Write(RWrite),
    Clunk(RClunk),
    Remove(RRemove),
    Walk(RWalk),
}

impl<'a> Response<'a> {
    pub fn deserialize(buf: &'a [u8]) -> Option<Self> {
        assert!(
            buf.len() > 5,
            "buf not long enough for message type and tag"
        );
        let mut reader = ByteReader::new(buf);
        let len = reader.read_u32()?;
        let typ = reader.read_u8()?;
        let typ = P9MsgType::try_from(typ).ok()?;
        reader.buf = &reader.buf[0..len as usize];
        match typ {
            P9MsgType::RVersion => {
                let tag = reader.read_u16()?;
                let msize = reader.read_u32()?;
                let version = reader.read_str()?;
                Some(Response::Version(RVersion {
                    msize,
                    version,
                    tag,
                }))
            }
            P9MsgType::RAuth => todo!("9p RAuth"),
            P9MsgType::RAttach => {
                let tag = reader.read_u16()?;
                let qid = P9Qid::deserialize(&mut reader)?;
                Some(Response::Attach(RAttach { tag, qid }))
            }
            P9MsgType::RError => {
                let tag = reader.read_u16()?;
                let ename_len = reader.read_u16()?;
                let ename = reader.read_slice(ename_len as usize)?;
                let ename = core::str::from_utf8(ename).unwrap();
                Some(Response::Error(RError { tag, ename }))
            }
            P9MsgType::RFlush => {
                let tag = reader.read_u16()?;
                Some(Response::Flush(RFlush { tag }))
            }
            P9MsgType::RWalk => {
                let tag = reader.read_u16()?;
                let nwqids = reader.read_u16()?;
                let mut qids = [P9Qid {
                    typ: P9QidType::File,
                    version: 0,
                    path: 0,
                }; MAXWELEM];
                for i in 0..MAXWELEM as u16 {
                    if i < nwqids {
                        qids[i as usize] = P9Qid::deserialize(&mut reader)?;
                    }
                }
                Some(Response::Walk(RWalk { tag, nwqids, qids }))
            }
            P9MsgType::ROpen => {
                let tag = reader.read_u16()?;
                let qid = P9Qid::deserialize(&mut reader)?;
                let iounit = reader.read_u32()?;
                Some(Response::Open(ROpen { tag, qid, iounit }))
            }
            P9MsgType::RCreate => todo!("9P RCreate"),
            P9MsgType::RRead => {
                let tag = reader.read_u16()?;
                let count = reader.read_u32()?;
                let data = reader.read_slice(count as usize)?;
                Some(Response::Read(RRead { tag, data }))
            }
            P9MsgType::RWrite => {
                let tag = reader.read_u16()?;
                let count = reader.read_u32()?;
                Some(Response::Write(RWrite { tag, count }))
            }
            P9MsgType::RClunk => {
                let tag = reader.read_u16()?;
                Some(Response::Clunk(RClunk { tag }))
            }
            P9MsgType::RRemove => {
                let tag = reader.read_u16()?;
                Some(Response::Remove(RRemove { tag }))
            }
            P9MsgType::RStat => todo!("9P RStat"),
            P9MsgType::RWStat => todo!("9P RWStat"),
            _ => panic!("invalid message"),
        }
    }
}

#[derive(Debug)]
pub struct RError<'a> {
    pub tag: u16,
    pub ename: &'a str,
}

#[derive(Debug)]
pub struct TVersion<'a> {
    pub msize: u32,
    pub version: &'a str,
}

impl TVersion<'_> {
    pub fn serialize(&self, mut req: P9RequestBuilder) {
        req.write_type(P9MsgType::TVersion);
        req.write_u16(!0);
        req.write_u32(self.msize);
        req.write_str(self.version);
        req.finish();
    }
}

#[derive(Debug)]
pub struct RVersion<'a> {
    pub msize: u32,
    pub version: &'a str,
    pub tag: u16,
}

#[derive(Debug)]
pub struct TAttach<'a> {
    pub tag: u16,
    pub fid: u32,
    pub afid: u32,
    pub uname: &'a str,
    pub aname: &'a str,
}

impl TAttach<'_> {
    pub fn serialize(&self, mut req: P9RequestBuilder) {
        req.write_type(P9MsgType::TAttach);
        req.write_u16(self.tag);
        req.write_u32(self.fid);
        req.write_u32(self.afid);
        req.write_str(self.uname);
        req.write_str(self.aname);
        req.finish();
    }
}

#[derive(Debug)]
pub struct RAttach {
    pub tag: u16,
    pub qid: P9Qid,
}

#[derive(Debug)]
pub struct TWalk<'a> {
    pub tag: u16,
    pub fid: u32,
    pub newfid: u32,
    pub wnames: &'a [&'a str],
}

impl TWalk<'_> {
    pub fn serialize(&self, mut req: P9RequestBuilder) {
        req.write_type(P9MsgType::TWalk);
        req.write_u16(self.tag);
        req.write_u32(self.fid);
        req.write_u32(self.newfid);
        req.write_u16(self.wnames.len() as u16);
        for wname in self.wnames {
            req.write_str(wname);
        }
        req.finish()
    }
}

pub struct RWalk {
    pub tag: u16,
    nwqids: u16,
    qids: [P9Qid; MAXWELEM],
}

impl RWalk {
    pub fn qids(&self) -> &[P9Qid] {
        &self.qids[..self.nwqids as usize]
    }
}

impl Debug for RWalk {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RWalk")
            .field("tag", &self.tag)
            .field("qids", &self.qids())
            .finish()
    }
}

#[derive(Debug)]
pub struct TRead {
    pub tag: u16,
    pub fid: u32,
    pub offset: u64,
    pub count: u32,
}

impl TRead {
    pub fn serialize(&self, mut req: P9RequestBuilder) {
        req.write_type(P9MsgType::TRead);
        req.write_u16(self.tag);
        req.write_u32(self.fid);
        req.write_u64(self.offset);
        req.write_u32(self.count);
        req.finish();
    }
}

#[derive(Debug)]
pub struct RRead<'d> {
    pub tag: u16,
    pub data: &'d [u8],
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
#[allow(dead_code, non_camel_case_types)]
pub enum P9FileMode {
    OREAD = 0,
    OWRITE = 1,
    ORDWR = 2,
    OEXEC = 3,
}

bitflags! {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct P9FileFlags: u8 {
        const OTRUNC = 0x10;
        const ORCLOSE = 0x40;
    }
}

#[derive(Debug)]
pub struct TOpen {
    pub tag: u16,
    pub fid: u32,
    pub mode: P9FileMode,
    pub flags: P9FileFlags,
}

impl TOpen {
    pub fn serialize(&self, mut req: P9RequestBuilder) {
        req.write_type(P9MsgType::TOpen);
        req.write_u16(self.tag);
        req.write_u32(self.fid);
        req.write_u8(self.mode as u8 | self.flags.bits());
        req.finish();
    }
}

#[derive(Debug)]
pub struct RFlush {
    pub tag: u16,
}

#[derive(Debug)]
pub struct ROpen {
    pub tag: u16,
    pub qid: P9Qid,
    pub iounit: u32,
}

#[derive(Debug)]
pub struct RWrite {
    pub tag: u16,
    pub count: u32,
}

#[derive(Debug)]
pub struct TClunk {
    pub tag: u16,
    pub fid: u32,
}
impl TClunk {
    pub fn serialize(&self, mut req: P9RequestBuilder) {
        req.write_type(P9MsgType::TClunk);
        req.write_u16(self.tag);
        req.write_u32(self.fid);
        req.finish();
    }
}

#[derive(Debug)]
pub struct RClunk {
    pub tag: u16,
}

#[derive(Debug)]
pub struct RRemove {
    pub tag: u16,
}

#[derive(Debug)]
#[allow(unused)]
pub struct Stat<'a> {
    typ: u16,
    dev: u32,
    pub qid: P9Qid,
    pub mode: u32,
    pub atime: u32,
    pub mtime: u32,
    pub length: u64,
    pub name: &'a str,
    pub uid: &'a str,
    pub gid: &'a str,
    pub muid: &'a str,
}

impl<'a> Stat<'a> {
    pub fn deserialize(reader: &mut ByteReader<'a>) -> Option<Self> {
        let size = reader.read_u16()?;
        let data = reader.read_slice(size as usize)?;
        let mut reader = ByteReader::new(data);
        let typ = reader.read_u16().unwrap();
        let dev = reader.read_u32().unwrap();
        let qid = P9Qid::deserialize(&mut reader).unwrap();
        let mode = reader.read_u32().unwrap();
        let atime = reader.read_u32().unwrap();
        let mtime = reader.read_u32().unwrap();
        let length = reader.read_u64().unwrap();
        let name = reader.read_str().unwrap();
        let uid = reader.read_str().unwrap();
        let gid = reader.read_str().unwrap();
        let muid = reader.read_str().unwrap();
        Some(Self {
            typ,
            dev,
            qid,
            mode,
            atime,
            mtime,
            length,
            name,
            uid,
            gid,
            muid,
        })
    }
}
