use core::ops::Range;

use librust::println;

use crate::{
    drivers::p9::{ByteReader, Stat},
    read::Reader,
    FS,
};

use super::Command;

pub struct Ls;

// move back to virtio9p
pub struct DirReader<'r, 'b> {
    reader: &'r mut dyn Reader,
    buf: &'b mut [u8],
    buf_range: Range<usize>,
}

type StatBytes<'a> = &'a [u8];

fn try_read_stat_and_cont<'a>(buf: &'a [u8]) -> Option<(StatBytes<'a>, usize)> {
    let mut reader = ByteReader::new(&buf);
    let _stat = Stat::deserialize(&mut reader)?;
    let read = buf.len() - reader.remaining_data().len();
    let bytes = &buf[0..read];
    Some((bytes, read))
}

impl<'r, 'b> DirReader<'r, 'b> {
    fn try_buf_read<'s>(&'s mut self) -> Option<StatBytes<'s>> {
        if let Some((stat, read)) = try_read_stat_and_cont(&self.buf[self.buf_range.clone()]) {
            self.buf_range.start += read;
            return Some(stat);
        }
        None
    }

    fn move_buf_to_front(&mut self) {
        self.buf.copy_within(self.buf_range.clone(), 0);
        self.buf_range.end = self.buf_range.end - self.buf_range.start;
        self.buf_range.start = 0;
    }

    fn fill_buf(&mut self) -> bool {
        let read = self
            .reader
            .read(&mut self.buf[self.buf_range.end..])
            .unwrap();
        if read == 0 {
            return false;
        }
        self.buf_range.end += read;
        return true;
    }

    fn read_entry<'s>(&'s mut self, buf: &mut [u8]) -> Option<usize> {
        if let Some(stat) = self.try_buf_read() {
            buf[0..stat.len()].clone_from_slice(stat);
            return Some(stat.len());
        }

        self.move_buf_to_front();
        if !self.fill_buf() {
            return None;
        }

        if let Some(stat) = self.try_buf_read() {
            buf[0..stat.len()].clone_from_slice(stat);
            return Some(stat.len());
        }
        return None;
    }
}

impl Command for Ls {
    fn get_name(&self) -> &'static str {
        "ls"
    }

    fn get_summary(&self) -> &'static str {
        "list directory"
    }

    fn execute(&self, args: &str) -> Result<(), &'static str> {
        let mut p9 = FS.0.borrow_mut();
        let p9 = p9.as_mut().unwrap();
        // TODO: remove this intermediate buffer, this should be the p9 protocol buffer
        let mut buf = [0u8; 256];
        let mut stat_buf = [0u8; 128];
        let mut dir_reader = DirReader {
            reader: &mut p9.read_file(&[]).unwrap(),
            buf: &mut buf,
            buf_range: 0..0,
        };
        loop {
            let Some(stat) = dir_reader.read_entry(&mut stat_buf) else {
                break;
            };
            let stat = Stat::deserialize(&mut ByteReader::new(&stat_buf)).unwrap();
            println!("{}", stat.name);
        }

        Ok(())
    }
}
