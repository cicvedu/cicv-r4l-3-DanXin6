// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.

use core::result::Result::Err;

use kernel::prelude::*;
use kernel::sync::Mutex;
use kernel::{chrdev, file};

const GLOBALMEM_SIZE: usize = 0x1000;

module! {
    type: RustChrdev,
    name: "rust_chrdev",
    author: "Rust for Linux Contributors",
    description: "Rust character device sample",
    license: "GPL",
}

/// 当前所有的数据都存在一起， 暂不支持多线程， 也不支持多个文件同时操
static GLOBALMEM_BUF: Mutex<[u8; GLOBALMEM_SIZE]> = unsafe { Mutex::new([0u8; GLOBALMEM_SIZE]) };
static GLOBALMEM_OFFSET: Mutex<usize> = unsafe { Mutex::new(0) };
struct RustFile {
    inner: &'static Mutex<[u8; GLOBALMEM_SIZE]>,
    offset: &'static Mutex<usize>,
}

#[vtable]
impl file::Operations for RustFile {
    type Data = Box<Self>;

    fn open(_shared: &(), _file: &file::File) -> Result<Box<Self>> {
        Ok(Box::try_new(RustFile {
            inner: &GLOBALMEM_BUF,
            offset: &GLOBALMEM_OFFSET,
        })?)
    }

    fn write(
        this: &Self,
        _file: &file::File,
        reader: &mut impl kernel::io_buffer::IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {
        pr_info!("Rust character device sample (write)\n");
        let buf = reader.read_all()?;
        // if buf.is_empty() {
        //     return Ok(0);
        // }

        buf.iter().for_each(|b| pr_info!("{} ", b));

        pr_info!(
            "Rust character device sample (write) - write {} bytes\n",
            buf.len()
        );
        let mut globalmem = this.inner.lock();

        let offset = *this.offset.lock();
        if offset + buf.len() > GLOBALMEM_SIZE {
            return Err(ENOSR)
        }

        globalmem[offset..offset + buf.len()].copy_from_slice(&buf);
        // globalmem[..buf.len()].copy_from_slice(&buf);
        // *this.w_offset.lock() = buf.len();
        *this.offset.lock() += buf.len();

        pr_info!("Rust character device sample (write) - write offset: {}\n", *this.offset.lock());
        Ok(buf.len())
    }

    fn read(
        this: &Self,
        _file: &file::File,
        writer: &mut impl kernel::io_buffer::IoBufferWriter,
        _offset: u64,
    ) -> Result<usize> {
        pr_info!("Rust character device sample (read)\n");
        let globalmem = this.inner.lock();

        let len = *this.offset.lock();
        pr_info!("Rust character device sample (read) - read len: {}\n", writer.len());
        writer.write_slice(&globalmem[..len])?;

        *this.offset.lock() = 0;
        
        Ok(len)
    }
}

struct RustChrdev {
    _dev: Pin<Box<chrdev::Registration<2>>>,
}

impl kernel::Module for RustChrdev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust character device sample (init)\n");

        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;

        // Register the same kind of device twice, we're just demonstrating
        // that you can use multiple minors. There are two minors in this case
        // because its type is `chrdev::Registration<2>`
        chrdev_reg.as_mut().register::<RustFile>()?;
        chrdev_reg.as_mut().register::<RustFile>()?;

        Ok(RustChrdev { _dev: chrdev_reg })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust character device sample (exit)\n");
    }
}
