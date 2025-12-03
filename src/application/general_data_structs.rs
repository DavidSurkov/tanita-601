#[derive(Debug)]
pub struct Date {
    years: u16,
    months: u8,
    days: u8,
}

impl Date {
    pub fn from_string(date_dmy: &str) -> Option<Date> {
        let mut iterator = date_dmy.trim_matches('"').split('/');
        let d = iterator.next()?;
        let m = iterator.next()?;
        let y = iterator.next()?;

        if iterator.next().is_some() {
            println!("DATE PARSIG HAS SOME EXTRA VALUE");
            return None;
        }

        let days = d.parse::<u8>().ok()?;
        let months = m.parse::<u8>().ok()?;
        let years = y.parse::<u16>().ok()?;

        return Some(Date {
            days,
            months,
            years,
        });
    }

    pub fn to_srting(&self) -> String {
        format!("{}/{}/{}", self.years, self.months, self.days)
    }
}

#[derive(Debug)]
pub struct Time {
    hours: u8,
    minutes: u8,
    seconds: u8,
}

impl Time {
    pub fn from_string(time_hms: &str) -> Option<Time> {
        let mut iterator = time_hms.trim_matches('"').split(':');
        let h = iterator.next()?;
        let m = iterator.next()?;
        let s = iterator.next()?;

        if iterator.next().is_some() {
            return None;
        }

        let hours = h.parse::<u8>().ok()?;
        let minutes = m.parse::<u8>().ok()?;
        let seconds = s.parse::<u8>().ok()?;

        return Some(Time {
            hours,
            minutes,
            seconds,
        });
    }

    pub fn to_srting(&self) -> String {
        format!("{}:{}:{}", self.hours, self.minutes, self.seconds)
    }
}

#[derive(Debug)]
pub struct DateTime {
    date: Date,
    time: Time,
}

impl DateTime {
    pub fn from_string(date_dmy: &str, time_hms: &str) -> Option<DateTime> {
        match (Date::from_string(date_dmy), Time::from_string(time_hms)) {
            (Some(date), Some(time)) => Some(DateTime { date, time }),
            options => {
                println!("Datetime is unable to parse this shit: {:?}", options);
                return None;
            }
        }
    }

    pub fn to_string(&self) -> String {
        format!("{} {}", self.date.to_srting(), self.time.to_srting())
    }
}

#[derive(Debug, Clone)]
pub enum Gender {
    Male,
    Female,
    Other(u8),
}

impl Gender {
    pub fn to_string(&self) -> String {
        match self {
            Gender::Male => "Boy".to_string(),
            Gender::Female => "Girl".to_string(),
            Gender::Other(n) => format!("Unknown gender: {}", n),
        }
    }
}

impl From<u8> for Gender {
    fn from(code: u8) -> Self {
        match code {
            1 => Gender::Male,
            2 => Gender::Female,
            _ => Gender::Other(code),
        }
    }
}
