//! On Windows, the driver adb needs for the Orbit over USB.
//!
//! The Orbits don't tell Windows which driver their adb interface takes (they give Windows that hint only for file
//! transfer), and Orbit doesn't publish one, so Windows leaves the interface without a driver. Windows' own WinUSB
//! driver works with adb, and Device Manager can assign it by hand ("Let me pick", "Universal Serial Bus devices",
//! "WinUsb Device"); this assigns it the same way, to every adb interface that has no driver. Microsoft signs that
//! driver, so nothing needs signing, but assigning a driver takes administrator rights: the installer runs itself again
//! for it, with Windows' permission prompt, and only that copy runs as administrator.

/// The command-line argument for the copy that assigns the driver.
pub const ARGUMENT: &str = "--install-usb-driver";

#[cfg(windows)]
mod imp {
    use std::ptr::{null, null_mut};

    use windows_sys::core::GUID;
    use windows_sys::Win32::Devices::DeviceAndDriverInstallation::*;
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
    use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    /// The compatible ID of an Android device's adb interface (class ff, subclass 42, protocol 1).
    const ADB_INTERFACE: &str = "usb\\class_ff&subclass_42&prot_01";
    /// "Universal Serial Bus devices", the class of WinUSB devices.
    const USB_DEVICE_CLASS: GUID = GUID::from_u128(0x88bae032_5a81_49f0_bc3d_a4ff138216d6);

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// A device's registry property as text: each string of a multi-string, lowercase.
    unsafe fn strings(set: HDEVINFO, device: &mut SP_DEVINFO_DATA, property: u32) -> Vec<String> {
        let mut buffer = vec![0u8; 4096];
        let mut size = 0u32;
        if SetupDiGetDeviceRegistryPropertyW(set, device, property, null_mut(), buffer.as_mut_ptr(), buffer.len() as u32,
                                             &mut size) == FALSE {
            return Vec::new();
        }
        let units: Vec<u16> = buffer[..size as usize].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        units.split(|&u| u == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf16_lossy(s).to_lowercase()).collect()
    }

    /// Calls `each` for every adb interface plugged in that has no driver.
    unsafe fn for_each_without_driver(mut each: impl FnMut(HDEVINFO, &mut SP_DEVINFO_DATA)) -> Result<(), String> {
        let set = SetupDiGetClassDevsW(null(), null(), null_mut(), DIGCF_ALLCLASSES | DIGCF_PRESENT);
        if set as isize == INVALID_HANDLE_VALUE as isize {
            return Err("Windows couldn't list its devices.".into());
        }
        let mut index = 0;
        loop {
            let mut device: SP_DEVINFO_DATA = std::mem::zeroed();
            device.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
            if SetupDiEnumDeviceInfo(set, index, &mut device) == FALSE {
                break;
            }
            index += 1;
            let adb = strings(set, &mut device, SPDRP_COMPATIBLEIDS).iter().any(|id| id == ADB_INTERFACE);
            if adb && strings(set, &mut device, SPDRP_DRIVER).is_empty() {
                each(set, &mut device);
            }
        }
        SetupDiDestroyDeviceInfoList(set);
        Ok(())
    }

    /// Whether an adb interface is plugged in without a driver.
    pub fn needed() -> bool {
        let mut found = false;
        let _ = unsafe { for_each_without_driver(|_, _| found = true) };
        found
    }

    /// Assigns WinUSB to every adb interface without a driver. Needs administrator rights.
    pub fn install() -> Result<usize, String> {
        let mut installed = 0;
        let mut failure = None;
        unsafe {
            for_each_without_driver(|set, device| match assign_winusb(set, device) {
                Ok(()) => installed += 1,
                Err(e) => failure = Some(e),
            })?;
        }
        match failure {
            Some(e) if installed == 0 => Err(e),
            _ => Ok(installed),
        }
    }

    unsafe fn assign_winusb(set: HDEVINFO, device: &mut SP_DEVINFO_DATA) -> Result<(), String> {
        // Only Windows' own winusb.inf is looked in, and its driver may be chosen although the device doesn't name it.
        let mut params: SP_DEVINSTALL_PARAMS_W = std::mem::zeroed();
        params.cbSize = std::mem::size_of::<SP_DEVINSTALL_PARAMS_W>() as u32;
        if SetupDiGetDeviceInstallParamsW(set, device, &mut params) == FALSE {
            return Err("Windows couldn't prepare the driver.".into());
        }
        params.Flags |= DI_ENUMSINGLEINF;
        params.FlagsEx |= DI_FLAGSEX_ALLOWEXCLUDEDDRVS;
        let windows = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
        let inf = wide(&format!("{windows}\\INF\\winusb.inf"));
        params.DriverPath[..inf.len()].copy_from_slice(&inf);
        if SetupDiSetDeviceInstallParamsW(set, device, &params) == FALSE {
            return Err("Windows couldn't prepare the driver.".into());
        }
        // A device without a driver has no class yet; WinUSB's is "Universal Serial Bus devices".
        device.ClassGuid = USB_DEVICE_CLASS;
        if SetupDiBuildDriverInfoList(set, device, SPDIT_CLASSDRIVER) == FALSE {
            return Err("Windows couldn't find its WinUSB driver.".into());
        }
        let mut driver: SP_DRVINFO_DATA_V2_W = std::mem::zeroed();
        driver.cbSize = std::mem::size_of::<SP_DRVINFO_DATA_V2_W>() as u32;
        let found = SetupDiEnumDriverInfoW(set, device, SPDIT_CLASSDRIVER, 0, &mut driver) != FALSE;
        let result = if !found {
            Err("Windows couldn't find its WinUSB driver.".to_string())
        } else if SetupDiSetSelectedDriverW(set, device, &mut driver) == FALSE {
            Err("Windows couldn't choose its WinUSB driver.".to_string())
        } else {
            let mut restart = FALSE;
            if DiInstallDevice(null_mut(), set, device, &driver, 0, &mut restart) == FALSE {
                Err(format!("Windows couldn't install the WinUSB driver (error {}).",
                            windows_sys::Win32::Foundation::GetLastError()))
            } else {
                Ok(())
            }
        };
        SetupDiDestroyDriverInfoList(set, device, SPDIT_CLASSDRIVER);
        result
    }

    /// Runs this program again as administrator to assign the driver, after Windows asks the user; its exit code is
    /// the number of devices given the driver, or 1000 and more for a failure.
    pub fn install_elevated() -> Result<usize, String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let file = wide(&exe.to_string_lossy());
        let verb = wide("runas");
        let args = wide(super::ARGUMENT);
        unsafe {
            let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
            info.fMask = SEE_MASK_NOCLOSEPROCESS;
            info.lpVerb = verb.as_ptr();
            info.lpFile = file.as_ptr();
            info.lpParameters = args.as_ptr();
            info.nShow = SW_HIDE;
            if ShellExecuteExW(&mut info) == FALSE {
                return Err("The driver wasn't installed: Windows wasn't given permission.".into());
            }
            WaitForSingleObject(info.hProcess, INFINITE);
            let mut code = 0u32;
            GetExitCodeProcess(info.hProcess, &mut code);
            CloseHandle(info.hProcess);
            if code >= 1000 {
                Err("Windows couldn't install the WinUSB driver for the Orbit.".into())
            } else {
                Ok(code as usize)
            }
        }
    }
}

#[cfg(windows)]
pub use imp::{install, install_elevated, needed};

#[cfg(not(windows))]
pub fn needed() -> bool {
    false
}

#[cfg(not(windows))]
pub fn install() -> Result<usize, String> {
    Err("Only Windows needs this.".into())
}

#[cfg(not(windows))]
pub fn install_elevated() -> Result<usize, String> {
    Err("Only Windows needs this.".into())
}
