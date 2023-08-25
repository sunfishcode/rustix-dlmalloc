use crate::Allocator;
use core::ptr;
use rustix::mm::{MapFlags, MremapFlags, ProtFlags};
#[cfg(feature = "global")]
use rustix_futex_sync::lock_api::RawMutex as _;
#[cfg(feature = "global")]
use rustix_futex_sync::RawMutex;

/// System setting for Linux
pub struct System {
    _priv: (),
}

impl System {
    pub const fn new() -> System {
        System { _priv: () }
    }
}

#[cfg(feature = "global")]
static mut LOCK: RawMutex = RawMutex::INIT;

unsafe impl Allocator for System {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        let r = unsafe {
            rustix::mm::mmap_anonymous(
                ptr::null_mut(),
                size,
                ProtFlags::WRITE | ProtFlags::READ,
                MapFlags::PRIVATE,
            )
        };
        match r {
            Err(_) => (ptr::null_mut(), 0, 0),
            Ok(addr) => (addr as *mut u8, size, 0),
        }
    }

    #[cfg(target_os = "linux")]
    fn remap(&self, ptr: *mut u8, oldsize: usize, newsize: usize, can_move: bool) -> *mut u8 {
        let flags = if can_move {
            MremapFlags::MAYMOVE
        } else {
            MremapFlags::empty()
        };
        let r = unsafe { rustix::mm::mremap(ptr as *mut _, oldsize, newsize, flags) };
        match r {
            Err(_) => ptr::null_mut(),
            Ok(ptr) => ptr as *mut u8,
        }
    }

    #[cfg(target_os = "macos")]
    fn remap(&self, _ptr: *mut u8, _oldsize: usize, _newsize: usize, _can_move: bool) -> *mut u8 {
        ptr::null_mut()
    }

    #[cfg(target_os = "linux")]
    fn free_part(&self, ptr: *mut u8, oldsize: usize, newsize: usize) -> bool {
        unsafe {
            let r = rustix::mm::mremap(ptr as *mut _, oldsize, newsize, MremapFlags::empty());
            match r {
                Ok(_) => return true,
                Err(_) => {
                    rustix::mm::munmap(ptr.offset(newsize as isize) as *mut _, oldsize - newsize)
                        .is_ok()
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    fn free_part(&self, ptr: *mut u8, oldsize: usize, newsize: usize) -> bool {
        unsafe {
            rustix::mm::munmap(ptr.offset(newsize as isize) as *mut _, oldsize - newsize).is_ok()
        }
    }

    fn free(&self, ptr: *mut u8, size: usize) -> bool {
        unsafe { rustix::mm::munmap(ptr as *mut _, size).is_ok() }
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        true
    }

    fn allocates_zeros(&self) -> bool {
        true
    }

    fn page_size(&self) -> usize {
        4096
    }
}

#[cfg(feature = "global")]
pub fn acquire_global_lock() {
    unsafe { LOCK.lock() }
}

#[cfg(feature = "global")]
pub fn release_global_lock() {
    unsafe { LOCK.unlock() }
}
