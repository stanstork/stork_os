use crate::sync::mutex::SpinMutex;

use super::io;

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
            return value;
        }

        (value & 0xF) + ((value / 16) * 10)
    }

    pub fn read_minutes(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(2);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            return value;
        }

        (value & 0xF) + ((value / 16) * 10)
    }

    pub fn read_hours(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(4);
        let reg_b = self.read_rtc(0xB);

        let is_pm = (value & 0x80) != 0;

        if reg_b & 0x02 == 0 {
            return value;
        }

        let mut hours = (value & 0xF) + ((value / 16) * 10);

        if is_pm {
            hours += 12;
        }

        hours
    }

    pub fn read_weekday(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(6);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            return value;
        }

        (value & 0xF) + ((value / 16) * 10)
    }

    pub fn read_day_of_month(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(7);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            return value;
        }

        (value & 0xF) + ((value / 16) * 10)
    }

    pub fn read_month(&self) -> u8 {
        self.update_in_progress();
        let value = self.read_rtc(8);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            return value;
        }

        (value & 0xF) + ((value / 16) * 10)
    }

    pub fn read_year(&self) -> u16 {
        self.update_in_progress();
        let value = self.read_rtc(9);
        let reg_b = self.read_rtc(0xB);

        if reg_b & 0x04 == 0 {
            return value as u16;
        }

        (value & 0xF) as u16 + (((value / 16) * 10) as u16)
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
        unsafe {
            io::outb(0x70, reg);
            io::inb(0x71)
        }
    }

    fn update_in_progress(&self) {
        while self.read_rtc(0xA) & 0x80 != 0 {}
    }
}

pub static mut RTC: SpinMutex<RtClock> = SpinMutex::new(RtClock::new());
