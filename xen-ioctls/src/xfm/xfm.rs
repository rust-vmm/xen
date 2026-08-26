/*
 * Copyright 2022-23 Mathieu Poirier <mathieu.poirier@linaro.org>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

use std::{
    convert::TryInto,
    fs::OpenOptions,
    io::{Error, ErrorKind},
    os::unix::io::AsRawFd,
    ptr, thread, time,
};

use libc::{c_int, c_void, mmap, munmap, ENOENT, MAP_PRIVATE, MAP_SHARED, PROT_READ, PROT_WRITE};

use crate::{
    private::*,
    xfm::{types::*, xfm_types::*},
};

fn map_from_address(
    size: usize,
    prot: i32,
    flags: i32,
    offset: i64,
) -> Result<*mut c_void, std::io::Error> {
    let fd = OpenOptions::new()
        .read(true)
        .write(true)
        .open(HYPERCALL_PRIVCMD)?;

    // SAFETY: `fd` is a valid file descriptor, mmap sets errno on error.
    let vaddr = unsafe {
        mmap(ptr::null_mut(), size, prot, flags, fd.as_raw_fd(), offset).cast::<c_void>()
    };

    // Function mmap() returns -1 in case of error.  Casting to i16 or i64
    // yield the same result.
    if vaddr as i8 == -1 {
        return Err(Error::last_os_error());
    }

    Ok(vaddr)
}

/// # Safety
///
/// `addr` placement hint must be a valid otherwise unused address.
pub unsafe fn xenforeignmemory_map_resource(
    domid: u16,
    r#type: u32,
    id: u32,
    frame: u32,
    nr_frames: u64,
    addr: *mut c_void,
    prot: i32,
    flags: i32,
) -> Result<XenForeignMemoryResourceHandle, std::io::Error> {
    let mut privcmd_mmapresource = PrivCmdMmapResource {
        dom: domid,
        r#type,
        id,
        idx: frame,
        num: nr_frames,
        addr: ptr::null_mut(),
    };

    /* Check flags only contains POSIX defined values */
    if (flags & !(MAP_SHARED | MAP_PRIVATE)) != 0 {
        return Err(Error::other("Invalid flags"));
    }

    if addr.is_null() && nr_frames != 0 {
        privcmd_mmapresource.addr = map_from_address(
            (nr_frames << PAGE_SHIFT) as usize,
            PROT_READ | PROT_WRITE,
            flags | MAP_SHARED,
            0,
        )?;
    }

    // SAFETY: `privcmd_mmapresource` points to a valid privcmd_mmapresource value
    match unsafe {
        do_ioctl(
            IOCTL_PRIVCMD_MMAP_RESOURCE(),
            std::ptr::addr_of_mut!(privcmd_mmapresource).cast(),
        )
    } {
        Ok(_) => Ok(XenForeignMemoryResourceHandle {
            domid,
            r#type,
            id,
            frame: frame as u64,
            nr_frames: privcmd_mmapresource.num,
            addr: privcmd_mmapresource.addr,
            prot,
            flags,
        }),
        Err(e) => {
            if !addr.is_null() {
                // SAFETY: we set privcmd_mmapresource.addr to a mmap created value earlier.
                let unmap_result = unsafe {
                    munmap(
                        privcmd_mmapresource.addr,
                        (nr_frames << PAGE_SHIFT) as usize,
                    )
                };

                if unmap_result < 0 {
                    println!(
                        "Error {} unmapping vaddr: {:?}",
                        Error::last_os_error(),
                        privcmd_mmapresource.addr
                    );
                }
            }

            Err(e)
        }
    }
}

/// # Safety
///
/// `resource` must be a valid handle.
pub unsafe fn xenforeignmemory_unmap_resource(
    resource: &XenForeignMemoryResourceHandle,
) -> Result<(), std::io::Error> {
    // SAFETY: caller ensures `resource.addr` is valid.
    if unsafe { munmap(resource.addr, (resource.nr_frames << PAGE_SHIFT) as usize) } < 0 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

/// # Safety
///
/// `arr` and `err` must be non-null and have the correct size
unsafe fn retry_paged_pages(
    domid: u16,
    addr: *mut c_void,
    pages: u64,
    arr: *const u64,
    err: *mut c_int,
) -> Result<(), std::io::Error> {
    let mut i = 0;
    let mut batch_start = 0;

    while i < pages {
        /* Look for the first page that faulted */
        while i < pages {
            // SAFETY: caller ensures `err` is valid
            if unsafe { *err.add(i as usize) == -ENOENT } {
                batch_start = i;
                break;
            }

            i += 1;
        }

        let mut privcmd_mmapbatch_v2 = PrivCmdMmapBatchV2 {
            num: 1,
            dom: domid,
            addr: (addr as *mut u8).add((i << PAGE_SHIFT) as usize) as *mut c_void,
            // SAFETY: caller ensures `arr` is valid
            arr: arr.add(i as usize),
            // SAFETY: caller ensures `err` is valid
            err: err.add(i as usize),
        };

        i += 1;

        /* Try to lump as many_ consecutive_ faulted  pages in the same request */
        while i < pages {
            // SAFETY: caller ensures `err` is valid
            if unsafe { *err.add(i as usize) != -ENOENT } {
                break;
            }

            i += 1;
            privcmd_mmapbatch_v2.num += 1;
        }

        // SAFETY: caller ensures `err` and
        // `arr` pointers are valid
        match unsafe {
            do_ioctl(
                IOCTL_PRIVCMD_MMAPBATCH_V2(),
                std::ptr::addr_of_mut!(privcmd_mmapbatch_v2).cast(),
            )
        } {
            Ok(_) => {
                continue;
            }
            Err(err) if err.raw_os_error() == Some(libc::ENOENT) => {
                // Something went wrong with this batch, retry it
                i = batch_start;
                thread::sleep(time::Duration::from_micros(100));
                continue;
            }
            Err(err) => return Err(err),
        }
    }

    Ok(())
}

/// # Safety
///
/// `arr` and `err` must be non-null and have the correct size
pub unsafe fn xenforeignmemory_map(
    domid: u16,
    prot: i32,
    pages: u64,
    arr: *const u64,
    err: *mut c_int,
) -> Result<*mut c_void, std::io::Error> {
    let mut err_vec: Vec<c_int> = vec![0; pages as usize];
    let mut err_array: *mut c_int = err;
    let num: u32 = pages.try_into().map_err(|_| ErrorKind::InvalidInput)?;

    let addr: *mut c_void = map_from_address((pages << PAGE_SHIFT) as usize, prot, MAP_SHARED, 0)?;

    if err.is_null() {
        err_array = err_vec.as_mut_ptr();
    }

    let mut privcmd_mmapbatch_v2 = PrivCmdMmapBatchV2 {
        num,
        dom: domid,
        addr,
        arr,
        err: err_array,
    };

    // SAFETY: `privcmd_mmapbatch_v2` is a valid PrivCmdMmapBatchV2 struct
    match unsafe {
        do_ioctl(
            IOCTL_PRIVCMD_MMAPBATCH_V2(),
            std::ptr::addr_of_mut!(privcmd_mmapbatch_v2).cast(),
        )
    } {
        Ok(_) => Ok(addr),
        Err(e) => {
            if e.raw_os_error() != Some(libc::ENOENT) {
                return Err(e);
            }

            // SAFETY: caller ensures `err` and `arr` are valid pointers
            unsafe { retry_paged_pages(domid, addr, pages, arr, err_array).map(|_| addr) }
        }
    }
}

#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn xenforeignmemory_unmap(addr: *mut c_void, pages: u64) -> Result<(), std::io::Error> {
    // SAFETY: if `addr` is invalid, munmap simply sets EINVAL
    if unsafe { munmap(addr, (pages << PAGE_SHIFT) as usize) < 0 } {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}
