pub(crate) mod boot_sector;
pub(crate) mod directory_entry;
pub(crate) mod fat_driver;
pub(crate) mod long_directory_entry;

pub fn convert_to_fat_date(year: u16, month: u16, day: u16) -> u16 {
    ((year - 1980) << 9) | (month << 5) | day
}

pub fn convert_to_fat_time(hours: u16, minutes: u16) -> u16 {
    (hours << 11) | (minutes << 5) | (0) // Assuming seconds are zero
}
