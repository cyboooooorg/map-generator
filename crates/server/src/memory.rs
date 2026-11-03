//! Giving freed memory back to the system.
//!
//! A generation allocates and frees a few GB in many blocks across the
//! threads of its pool; glibc's allocator keeps much of it afterwards,
//! fragmented across its arenas, so the server held a few hundred MB it no
//! longer used between generations. The Docker image limits the arenas
//! (`MALLOC_ARENA_MAX=2`) and the server trims the heap after a generation
//! and after evicting worlds from the tile cache. mimalloc was measured
//! too: it kept more memory, and slowed generations down.

/// Returns the free memory of the heap to the system (`malloc_trim`). Takes
/// a few milliseconds on a large heap: call it off the async workers, after
/// large frees. Does nothing on other platforms than Linux with glibc.
pub fn release() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: malloc_trim has no preconditions; it only walks the
    // allocator's own free lists.
    unsafe {
        libc::malloc_trim(0);
    }
}
