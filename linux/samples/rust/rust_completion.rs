// SPDX-License-Identifier: GPL-2.0

//! Rust echo server sample.

use core::cell::UnsafeCell;
use core::default::Default;
use core::result::Result::Ok;

use kernel::bindings;
use kernel::sync::{Mutex, UniqueArc};
use kernel::{chrdev, file, prelude::*};

module! {
    type: RustCompletion,
    name: "rust_completion",
    description: "Rewriting C Completion using Rust",
    license: "GPL",
}

const GLOBALMEM_SIZE: usize = 0x1000;

static GLOBALMEM_BUF: Mutex<[u8; GLOBALMEM_SIZE]> = unsafe { Mutex::new([0u8; GLOBALMEM_SIZE]) };
static mut COMPLETION_FILE_OPS: Option<Pin<UniqueArc<RustCompletionFileOps>>> = None;

struct RustCompletionFileOps(UnsafeCell<bindings::completion>);

impl RustCompletionFileOps {
    /// New Ops
    fn new() -> Result<()> {
        unsafe {
            COMPLETION_FILE_OPS = Some(Pin::new(UniqueArc::try_new(RustCompletionFileOps(
                UnsafeCell::new(bindings::completion::default()),
            ))?))
        };

        Ok(())
    }

    /// 
    fn init_completion() {
        Self::new().unwrap();

        unsafe {
            if let Some(completion_file_ops) = &COMPLETION_FILE_OPS.as_mut() {
                bindings::init_completion(completion_file_ops.0.get())
            };
        }
    }

    fn write_completion() {
        unsafe {
            if let Some(completion_file_ops) = &COMPLETION_FILE_OPS.as_mut() {
                bindings::wait_for_completion(completion_file_ops.0.get());
            }
        }
    }

    fn completion() {
        unsafe {
            if let Some(completion_file_ops) = &COMPLETION_FILE_OPS.as_mut() {
                bindings::complete(completion_file_ops.0.get());
            }
        }
    }
}

struct RustCompletionFile {
    data: &'static Mutex<[u8; GLOBALMEM_SIZE]>,
}

#[vtable]
impl file::Operations for RustCompletionFile {
    type Data = Box<Self>;

    type OpenData = ();

    fn open(_context: &Self::OpenData, _file: &file::File) -> Result<Self::Data> {
        pr_info!("Rust for Linux Completion(open)\n");
        Ok(Box::try_new(RustCompletionFile {
            data: &GLOBALMEM_BUF,
        })?)
    }

    fn write(
        this: &Self,
        _file: &file::File,
        reader: &mut impl kernel::io_buffer::IoBufferReader,
        _offset: u64,
    ) -> Result<usize> {
        pr_info!("Rust for Linux Completion(write)\n");
        let mut globalmem = this.data.lock();
        let len = reader.len();
        globalmem[0] = len as u8;
        reader.read_slice(&mut globalmem[1..=len])?;

        RustCompletionFileOps::completion();
        pr_info!("Rust for Linux Completion(complete)\n");

        Ok(len as usize)
    }

    fn read(
        this: &Self,
        _file: &file::File,
        writer: &mut impl kernel::io_buffer::IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        pr_info!("Rust for Linux Completion(read)\n");
        if writer.is_empty() || offset > 0 {
            return Ok(0);
        }

        RustCompletionFileOps::write_completion();
        let globalmem = this.data.lock();
        let len = globalmem[0] as usize;
        writer.write_slice(&globalmem[1..=len])?;

        Ok(len as usize)
    }
}

struct RustCompletion {
    _dev: Pin<Box<chrdev::Registration<1>>>,
}

impl kernel::Module for RustCompletion {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust for Linux Completion(init)");

        RustCompletionFileOps::init_completion();

        let mut completion_reg = chrdev::Registration::new_pinned(name, 0, module)?;
        completion_reg.as_mut().register::<RustCompletionFile>()?;

        Ok(RustCompletion {
            _dev: completion_reg,
        })
    }
}

impl Drop for RustCompletion {
    fn drop(&mut self) {
        pr_info!("Rust for Linux Completion(exit)");
    }
}
