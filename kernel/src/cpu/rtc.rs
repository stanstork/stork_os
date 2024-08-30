use super::io;
use crate::sync::mutex::SpinMutex;

pub struct RtClock {}

impl RtClock {
    pub const fn new() -> Self {
        RtClock {}
    }

    pub fn read_seconds(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(0);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            // BCD mode
            return (value & 0xF) + ((value / 16) * 10);
        }

        value
    }

    pub fn read_minutes(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(2);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            // BCD mode
            return (value & 0xF) + ((value / 16) * 10);
        }

        value
    }

    pub fn read_hours(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(4);
        let reg_b = self.read_rtc(0xB);

        let mut hours = value;
        if reg_b & 0x02 == 0 && (value & 0x80) != 0 {
            // 12-hour mode and PM flag is set
            hours = ((value & 0xF) + ((value / 16) * 10)) + 12; // Convert to 24-hour format
        } else if reg_b & 0x04 == 0 {
            // BCD mode
            hours = (value & 0xF) + ((value / 16) * 10);
        }

        hours & 0x7F // Ensure we strip the PM bit for 24-hour mode
    }

    pub fn read_weekday(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(6);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            // BCD mode
            return (value & 0xF) + ((value / 16) * 10);
        }

        value
    }

    pub fn read_day_of_month(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(7);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            // BCD mode
            return (value & 0xF) + ((value / 16) * 10);
        }

        value
    }

    pub fn read_month(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(8);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            // BCD mode
            return (value & 0xF) + ((value / 16) * 10);
        }

        value
    }

    pub fn read_year(&self) -> u16 {
        self.update_in_progress();
        let value = self.read_rtc(9);
        let reg_b = self.read_rtc(0xB);

        let mut year = if reg_b & 0x04 == 0 {
            // BCD mode
            (value & 0xF) + ((value / 16) * 10)
        } else {
            value
        } as u16;

        // Adjust year to the correct century
        if year < 80 {
            // Assuming year is in the range 00-79, add 2000
            year += 2000;
        } else {
            // Assuming year is in the range 80-99, add 1900
            year += 1900;
        }

        year
    }

    pub fn read_time(&self) -> (u8, u8, u8) {
        (self.read_hours(), self.read_minutes(), self.read_seconds())
    }

    pub fn read_date(&self) -> (u8, u8, u16) {
        (
            self.read_day_of_month(),
            self.read_month(),
            self.read_year(),
        )
    }

    fn read_rtc(&self, reg: u8) -> u8 {
        io::outb(0x70, reg);
        io::inb(0x71)
    }

    fn update_in_progress(&self) {
        while self.read_rtc(0xA) & 0x80 != 0 {}
    }
}

pub static mut RTC: SpinMutex<RtClock> = SpinMutex::new(RtClock::new());
