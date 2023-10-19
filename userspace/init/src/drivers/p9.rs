use core::mem;

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

#[derive(Debug)]
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
        let (a, rest) = self.buf.split_at(mem::size_of::<u16>());
        let v = u16::from_le_bytes(a.try_into().unwrap());
        self.buf = rest;
        return Some(v);
    }

    pub fn read_u32(&mut self) -> Option<u32> {
        let (a, rest) = self.buf.split_at(mem::size_of::<u32>());
        let v = u32::from_le_bytes(a.try_into().unwrap());
        self.buf = rest;
        return Some(v);
    }

    pub fn read_u64(&mut self) -> Option<u64> {
        let (a, rest) = self.buf.split_at(mem::size_of::<u64>());
        let v = u64::from_le_bytes(a.try_into().unwrap());
        self.buf = rest;
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
}

pub enum Response<'a> {
    Version(RVersion<'a>),
    Attach(RAttach),
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
                let version_len = reader.read_u16()?;
                let version = reader.read_slice(version_len as usize)?;
                let version = core::str::from_utf8(version).unwrap();
                Some(Response::Version(RVersion {
                    msize,
                    version,
                    tag,
                }))
            }
            P9MsgType::RAuth => todo!(),
            P9MsgType::RAttach => {
                let tag = reader.read_u16()?;
                let qid = P9Qid::deserialize(&mut reader)?;
                Some(Response::Attach(RAttach { tag, qid }))
            }
            P9MsgType::RError => todo!(),
            P9MsgType::RFlush => todo!(),
            P9MsgType::RWalk => todo!(),
            P9MsgType::ROpen => todo!(),
            P9MsgType::RCreate => todo!(),
            P9MsgType::RRead => todo!(),
            P9MsgType::RWrite => todo!(),
            P9MsgType::RClunk => todo!(),
            P9MsgType::RRemove => todo!(),
            P9MsgType::RStat => todo!(),
            P9MsgType::RWStat => todo!(),
            _ => panic!("invalid message"),
        }
    }
}

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

pub struct RVersion<'a> {
    pub msize: u32,
    pub version: &'a str,
    pub tag: u16,
}

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

pub struct RAttach {
    pub tag: u16,
    pub qid: P9Qid,
}
