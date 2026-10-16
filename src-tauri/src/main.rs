// No console window behind the app on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // The copy that Windows runs as administrator to assign the USB driver does only that.
    if std::env::args().any(|a| a == satellite_setup_lib::usb_driver::ARGUMENT) {
        let code = match satellite_setup_lib::usb_driver::install() {
            Ok(installed) => installed as i32,
            Err(e) => {
                eprintln!("{e}");
                1000
            }
        };
        std::process::exit(code);
    }
    satellite_setup_lib::run()
}
