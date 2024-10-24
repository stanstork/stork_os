use crate::devices::timers::rtc::RTC;

pub(crate) mod boot;
pub(crate) mod dir_entry;
pub(crate) mod driver;
pub(crate) mod long_entry;

pub fn create_short_filename(name: &str) -> [u8; 11] {
    let mut short_name = [b' '; 11];
    let mut short_name_idx = 0;

    for c in name.chars() {
        if short_name_idx == 11 {
            break;
        }

        if c == '.' {
            short_name_idx = 8;
            continue;
        }

        short_name[short_name_idx] = c as u8;
        short_name_idx += 1;
    }

    short_name
}

pub fn calculate_checksum(short_name: &[u8]) -> u8 {
    short_name.iter().fold(0, |checksum, &byte| {
        ((checksum & 1) << 7).wrapping_add((checksum >> 1).wrapping_add(byte))
    })
}

pub fn get_current_fat_time_date() -> (u16, u16) {
    let (hour, minute, _) = unsafe { RTC.lock().read_time() };
    let (day, month, year) = unsafe { RTC.lock().read_date() };

    let creation_time = convert_to_fat_time(hour as u16, minute as u16);
    let creation_date = convert_to_fat_date(year as u16, month as u16, day as u16);

    (creation_time, creation_date)
}

pub fn convert_to_fat_date(year: u16, month: u16, day: u16) -> u16 {
    ((year - 1980) << 9) | (month << 5) | day
}

pub fn convert_to_fat_time(hours: u16, minutes: u16) -> u16 {
    (hours << 11) | (minutes << 5) | (0) // Assuming seconds are zero
}
