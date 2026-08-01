//! Native macOS memory readings.
//!
//! Every value the app shows about memory comes from here, via `sysctl` and
//! `proc_pid_rusage` rather than by parsing `vm_stat` or `memory_pressure` output. All
//! `unsafe` in the project lives in this module, and every accessor returns `Option` so
//! that one unavailable metric cannot blank the rest of the panel.

use std::ffi::CString;
use std::mem;

use serde::Serialize;

/// Values from `<dispatch/source.h>`: DISPATCH_MEMORYPRESSURE_NORMAL / _WARN / _CRITICAL.
const PRESSURE_NORMAL: libc::c_int = 1;
const PRESSURE_WARN: libc::c_int = 2;
const PRESSURE_CRITICAL: libc::c_int = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Pressure {
    Normal,
    Warning,
    Critical,
    #[default]
    Unknown,
}

impl Pressure {
    pub fn from_level(level: libc::c_int) -> Self {
        match level {
            PRESSURE_NORMAL => Pressure::Normal,
            PRESSURE_WARN => Pressure::Warning,
            PRESSURE_CRITICAL => Pressure::Critical,
            _ => Pressure::Unknown,
        }
    }
}

/// Reads a fixed-size `sysctl` value by name. Returns `None` when the name is unknown on
/// this kernel or the kernel reports a different size than the type expects — a size
/// mismatch means the struct layout has changed and the bytes must not be trusted.
fn sysctl<T: Copy>(name: &str) -> Option<T> {
    let key = CString::new(name).ok()?;
    let mut value: T = unsafe { mem::zeroed() };
    let mut size = mem::size_of::<T>();

    let result = unsafe {
        libc::sysctlbyname(
            key.as_ptr(),
            &mut value as *mut T as *mut libc::c_void,
            &mut size,
            std::ptr::null_mut(),
            0,
        )
    };

    if result != 0 || size != mem::size_of::<T>() {
        return None;
    }
    Some(value)
}

/// Installed unified memory.
pub fn installed_bytes() -> Option<u64> {
    sysctl::<u64>("hw.memsize")
}

/// The kernel's own memory pressure signal — authoritative, unlike any percentage we
/// could compute ourselves.
pub fn pressure() -> Pressure {
    match sysctl::<libc::c_int>("kern.memorystatus_vm_pressure_level") {
        Some(level) => Pressure::from_level(level),
        None => Pressure::Unknown,
    }
}

/// Swap currently in use, from `vm.swapusage`.
pub fn swap_used_bytes() -> Option<u64> {
    sysctl::<libc::xsw_usage>("vm.swapusage").map(|usage| usage.xsu_used)
}

/// The process's physical footprint — the number Activity Monitor's Memory column shows.
///
/// On Apple Silicon with `-ngl all` this substantially undercounts a model's true cost,
/// because Metal buffers are attributed to the kernel as wired memory. It is reported
/// alongside system-wide figures, never as the model's total.
pub fn process_footprint_bytes(pid: u32) -> Option<u64> {
    let mut info: libc::rusage_info_v4 = unsafe { mem::zeroed() };

    let result = unsafe {
        libc::proc_pid_rusage(
            pid as libc::c_int,
            libc::RUSAGE_INFO_V4,
            &mut info as *mut libc::rusage_info_v4 as *mut libc::rusage_info_t,
        )
    };

    if result != 0 {
        return None;
    }
    Some(info.ri_phys_footprint)
}

/// Signal 0 performs the permission and existence checks without delivering anything.
pub fn process_exists(pid: u32) -> bool {
    unsafe { libc::kill(pid as libc::c_int, 0) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_levels_map_to_named_states() {
        assert_eq!(Pressure::from_level(1), Pressure::Normal);
        assert_eq!(Pressure::from_level(2), Pressure::Warning);
        assert_eq!(Pressure::from_level(4), Pressure::Critical);
    }

    #[test]
    fn unrecognised_pressure_levels_are_unknown_not_normal() {
        assert_eq!(Pressure::from_level(0), Pressure::Unknown);
        assert_eq!(Pressure::from_level(3), Pressure::Unknown);
        assert_eq!(Pressure::from_level(-1), Pressure::Unknown);
    }

    #[test]
    fn unknown_sysctl_names_return_none_rather_than_garbage() {
        assert_eq!(sysctl::<u64>("hw.definitely_not_a_real_key"), None);
    }

    #[test]
    fn installed_memory_is_plausible() {
        let installed = installed_bytes().expect("hw.memsize is always available on macOS");
        assert!(
            installed >= 2 * 1024 * 1024 * 1024,
            "implausibly small: {installed}"
        );
    }

    #[test]
    fn swap_and_pressure_are_readable_on_this_machine() {
        assert!(
            swap_used_bytes().is_some(),
            "vm.swapusage should be readable"
        );
        assert_ne!(pressure(), Pressure::Unknown, "pressure should be readable");
    }

    #[test]
    fn own_process_reports_a_footprint() {
        let footprint = process_footprint_bytes(std::process::id()).expect("own footprint");
        assert!(footprint > 0);
    }

    #[test]
    fn our_own_process_exists_and_an_absurd_pid_does_not() {
        assert!(process_exists(std::process::id()));
        assert!(!process_exists(0x7FFF_FFFF));
    }

    #[test]
    fn a_pid_that_does_not_exist_reports_nothing() {
        assert_eq!(process_footprint_bytes(0x7FFF_FFFF), None);
    }
}
