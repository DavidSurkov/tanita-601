use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    usize,
};

use iced::{
    Length, Task, Theme,
    widget::{Column, Text, button, scrollable, text},
};

use rfd::AsyncFileDialog;

const PROFILE_FOLDER_NAME: &'static str = "SYSTEM";
const DATA_FOLDER_NAME: &'static str = "DATA";
const DATA_FILE_NAME_PREFIX: &'static str = "DATA";
const PROFILE_FILE_NAME_PREFIX: &'static str = "PROF";
const CSV_EXTENTION_NAME: &'static str = ".CSV";

#[derive(Debug)]
pub enum TanitaValidationError {
    MissingDir(&'static str),
    NoFilesFound,
    Unpaired {
        missing_in_data: BTreeSet<u32>,
        missing_in_profile: BTreeSet<u32>,
    },
}

impl fmt::Display for TanitaValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TanitaValidationError::NoFilesFound => {
                write!(f, "No DATA or PROFILE files found")
            }
            TanitaValidationError::Unpaired {
                missing_in_data,
                missing_in_profile,
            } => {
                write!(
                    f,
                    "unpaired indices: missing in DATA {:?}, missing in PROF {:?}",
                    missing_in_data, missing_in_profile
                )
            }
            TanitaValidationError::MissingDir(name) => {
                write!(f, "Missing required dir: {}", name)
            }
        }
    }
}

impl Error for TanitaValidationError {}

pub type TanitaResult<T> = Result<T, TanitaValidationError>;

async fn pick_folder() -> Option<PathBuf> {
    let file_handle = AsyncFileDialog::new()
        //TODO: set /home folder as root
        .set_title("Pick [GRAPHV1] folder isnide TANITA folder")
        .pick_folder()
        .await?;

    let path: PathBuf = file_handle.into();

    return Some(path);
}

#[derive(Debug)]
pub struct RawUserRecord {
    index: u32,
    profile: ProfRaw,
    data: Vec<DataRaw>,
}

struct TanitaParser {
    root_dir: PathBuf,
}

impl TanitaParser {
    fn parse_u8(s: &str) -> u8 {
        s.parse::<u8>().unwrap_or(0)
    }
    fn parse_u16(s: &str) -> u16 {
        s.parse::<u16>().unwrap_or(0)
    }
    fn parse_f32(s: &str) -> f32 {
        s.parse::<f32>().unwrap_or(0.0)
    }
    fn unquote(s: &str) -> String {
        let t = s.trim();
        t.strip_prefix('"')
            .and_then(|t| t.strip_suffix('"'))
            .unwrap_or(t)
            .to_string()
    }

    pub fn get_raw_users_records(&self) -> Vec<RawUserRecord> {
        let data_folder = self.require_dir(&self.root_dir, DATA_FOLDER_NAME).unwrap();
        let system_folder = self
            .require_dir(&self.root_dir, PROFILE_FOLDER_NAME)
            .unwrap();
        let data_files = self.collect_files(&data_folder);
        let prof_files = self.collect_files(&system_folder);
        let mut tanita_pairs: Vec<TanitaPair> = Vec::with_capacity(prof_files.len());

        for (file_num, profile_file) in prof_files {
            if data_files[&file_num].exists() {
                tanita_pairs.push(TanitaPair {
                    index: file_num,
                    profile: profile_file,
                    //TODO: why do i need to clone this one but not profile?
                    data: data_files[&file_num].clone(),
                });
            } else {
                panic!("profile and data do not match");
            }
        }

        let mut users_records = Vec::with_capacity(tanita_pairs.len());

        //Now we need to read all those files and parse data in it;
        for pair in tanita_pairs {
            let prof_file_content = pair.get_profile_file_content();
            let data_file_content = pair.get_data_file_content();
            let first_profile_line = prof_file_content.lines().collect::<Vec<&str>>()[0];

            let mut raw_user_record = RawUserRecord {
                index: pair.index,
                data: Vec::new(),
                profile: ProfRaw::from_csv_row(first_profile_line),
            };

            for data in data_file_content.lines() {
                raw_user_record.data.push(DataRaw::from_csv_row(data));
            }
            users_records.push(raw_user_record);
        }
        return users_records;
    }
    fn require_dir(&self, p: &PathBuf, name: &'static str) -> TanitaResult<PathBuf> {
        let dir = p.join(name);
        if dir.is_dir() {
            Ok(dir)
        } else {
            Err(TanitaValidationError::MissingDir(name))
        }
    }

    fn get_index(&self, file_name: &str) -> Option<u32> {
        let name = file_name.to_ascii_uppercase();
        let name_wihtout_extention = name.strip_suffix(CSV_EXTENTION_NAME)?;
        let digits = if let Some(d) = name_wihtout_extention.strip_prefix(DATA_FILE_NAME_PREFIX) {
            d
        } else if let Some(d) = name_wihtout_extention.strip_prefix(PROFILE_FILE_NAME_PREFIX) {
            d
        } else {
            return None;
        };
        digits.parse().ok()
    }

    fn collect_files(&self, dir: &Path) -> BTreeMap<u32, PathBuf> {
        let mut collecton = BTreeMap::new();
        let read = fs::read_dir(dir);
        match read {
            Ok(read_result) => {
                for entry in read_result.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if let Some(idx) = self.get_index(file_name) {
                            collecton.insert(idx, entry.path());
                        }
                    }
                }
            }
            Err(_) => {
                // todo!();
                println!("Folder {:?} is wrong", dir)
                // TODO: handle wrong folder
            }
        }
        collecton
    }
}

#[derive(Debug, Clone)]
pub struct TanitaPair {
    index: u32,
    profile: PathBuf,
    data: PathBuf,
}

impl TanitaPair {
    pub fn get_profile_file_content(&self) -> String {
        fs::read_to_string(self.profile.clone()).unwrap()
    }

    pub fn get_data_file_content(&self) -> String {
        fs::read_to_string(self.data.clone()).unwrap()
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

#[derive(Debug, Clone, Default)]
pub struct ProfRaw {
    /// `MO` — device model, e.g., "BC-601".
    pub model: String,
    /// `DB` — date of birth as printed by device, e.g., "14/06/1991".
    /// Keep as string here (std-only; parse later if you wish).
    pub birth_date_dmy: String,
    /// `Bt` — body/athlete mode code (device-specific numeric code).
    /// Values vary (0..=8 observed). Keep raw for now.
    pub body_type_code: u8,
    /// `GE` — gender/sex code, usually 1=Male, 2=Female on Tanita.
    pub gender_code: u8,
    /// `Hm` — height in centimeters, e.g., 175.0
    pub height_cm: f32,
    /// `AL` — activity level code (device menu selection).
    pub activity_level_code: u8,
    /// `CS` — checksum / record code reported at the end (often hex-like).
    pub checksum: String,
}

impl ProfRaw {
    pub fn from_csv_row(row: &str) -> ProfRaw {
        let data_entries: Vec<&str> = row.split(',').collect();
        let mut profile_raw = ProfRaw::default();

        let mut key_pointer = 0;
        while key_pointer < data_entries.len() {
            let key = data_entries[key_pointer];
            let value = data_entries[key_pointer + 1];

            match key {
                "MO" => profile_raw.model = TanitaParser::unquote(value),
                "DB" => profile_raw.birth_date_dmy = TanitaParser::unquote(value),
                "Bt" => profile_raw.body_type_code = TanitaParser::parse_u8(value),
                "GE" => profile_raw.gender_code = TanitaParser::parse_u8(value),
                "Hm" => profile_raw.height_cm = TanitaParser::parse_f32(value),
                "AL" => profile_raw.activity_level_code = TanitaParser::parse_u8(value),
                "CS" => profile_raw.checksum = TanitaParser::unquote(value),

                _ => {
                    println!("[Profile] Some extra key: {:?} and value: {:?}", key, value);
                }
            }
            key_pointer = key_pointer + 2;
        }

        return profile_raw;
    }
}

#[derive(Debug, Clone, Default)]
pub struct DataRaw {
    // --- Identity / timestamp ---
    /// `MO` Model string (often "BC-601" even on BC-603 FS).
    pub model: String,
    /// `DT` Measurement date "dd/mm/yyyy".
    pub date_dmy: String,
    /// `Ti` Measurement time "hh:mm:ss".
    pub time_hms: String,

    // --- Profile echoes (state at measurement) ---
    /// `GE` Gender code (device numeric).
    pub gender_code: u8,
    /// `AG` Age (years).
    pub age_years: u8,
    /// `Hm` Height (cm).
    pub height_cm: f32,
    /// `AL` Activity level code (device numeric).
    pub activity_level_code: u8,
    /// `Bt` Athlete/body-type mode code (device numeric).
    pub body_type_code: u8,

    // --- Core body metrics ---
    /// `Wk` Body mass (kg).
    pub weight_kg: f32,
    /// `MI` Body Mass Index.
    pub bmi: f32,
    /// `FW` Global fat (%).
    pub fat_percent: f32,

    // --- Segmental fat (%) ---
    /// `Fr` Arm fat (right) %.
    pub fat_right_arm_pct: f32,
    /// `Fl` Arm fat (left) %.
    pub fat_left_arm_pct: f32,
    /// `FR` Leg fat (right) %.
    pub fat_right_leg_pct: f32,
    /// `FL` Leg fat (left) %.
    pub fat_left_leg_pct: f32,
    /// `FT` Torso fat %.
    pub fat_trunk_pct: f32,

    // --- Muscle (%), whole + segments (present on newer rows) ---
    /// `mW` Global muscle %.
    pub muscle_percent: Option<f32>,
    /// `mr` Arm muscle (right) %.
    pub muscle_right_arm_pct: Option<f32>,
    /// `ml` Arm muscle (left) %.
    pub muscle_left_arm_pct: Option<f32>,
    /// `mR` Leg muscle (right) %.
    pub muscle_right_leg_pct: Option<f32>,
    /// `mL` Leg muscle (left) %.
    pub muscle_left_leg_pct: Option<f32>,
    /// `mT` Torso muscle %.
    pub muscle_trunk_pct: Option<f32>,

    // --- Other derived metrics ---
    /// `bw` Estimated bone mass (kg).
    pub bone_kg: Option<f32>,
    /// `ww` Global body water %.
    pub water_percent: Option<f32>,
    /// `IF` Visceral fat rating (integer-ish).
    pub visceral_fat_rating: Option<u8>,
    /// `rA` Estimated metabolic age (years).
    pub metabolic_age_years: Option<u8>,
    /// `rD` Daily calorie intake (DCI, kcal).
    pub daily_calorie_intake_kcal: Option<u16>,

    // --- Trailer ---
    /// `CS` Frame/check code (changes per entry; keep as-is).
    pub checksum: String,

    // --- Catch-all for future tags (lossless) ---
    pub extras: Vec<(String, String)>,
}

impl DataRaw {
    pub fn from_csv_row(row: &str) -> DataRaw {
        let data_entries: Vec<&str> = row.split(',').collect();
        let mut data_raw = DataRaw::default();

        let mut key_pointer = 0;
        while key_pointer < data_entries.len() {
            let key = data_entries[key_pointer];
            let value = data_entries[key_pointer + 1];

            match key {
                "MO" => data_raw.model = TanitaParser::unquote(value),
                "DT" => data_raw.date_dmy = TanitaParser::unquote(value),
                "Ti" => data_raw.time_hms = TanitaParser::unquote(value),
                "GE" => data_raw.gender_code = TanitaParser::parse_u8(value),
                "AG" => data_raw.age_years = TanitaParser::parse_u8(value),
                "Hm" => data_raw.height_cm = TanitaParser::parse_f32(value),

                "AL" => data_raw.activity_level_code = TanitaParser::parse_u8(value),
                "Bt" => data_raw.body_type_code = TanitaParser::parse_u8(value),
                "Wk" => data_raw.weight_kg = TanitaParser::parse_f32(value),
                "MI" => data_raw.bmi = TanitaParser::parse_f32(value),

                "FW" => data_raw.fat_percent = TanitaParser::parse_f32(value),
                "Fr" => data_raw.fat_right_arm_pct = TanitaParser::parse_f32(value),
                "Fl" => data_raw.fat_left_arm_pct = TanitaParser::parse_f32(value),
                "FR" => data_raw.fat_right_leg_pct = TanitaParser::parse_f32(value),
                "FL" => data_raw.fat_left_leg_pct = TanitaParser::parse_f32(value),
                "FT" => data_raw.fat_trunk_pct = TanitaParser::parse_f32(value),

                "mW" => data_raw.muscle_percent = Some(TanitaParser::parse_f32(value)),
                "ml" => data_raw.muscle_left_arm_pct = Some(TanitaParser::parse_f32(value)),
                "mr" => data_raw.muscle_right_arm_pct = Some(TanitaParser::parse_f32(value)),
                "mR" => data_raw.muscle_right_leg_pct = Some(TanitaParser::parse_f32(value)),
                "mL" => data_raw.muscle_left_leg_pct = Some(TanitaParser::parse_f32(value)),
                "mT" => data_raw.muscle_trunk_pct = Some(TanitaParser::parse_f32(value)),

                "bw" => data_raw.bone_kg = Some(TanitaParser::parse_f32(value)),
                "ww" => data_raw.water_percent = Some(TanitaParser::parse_f32(value)),
                "IF" => data_raw.visceral_fat_rating = Some(TanitaParser::parse_u8(value)),
                "rA" => data_raw.metabolic_age_years = Some(TanitaParser::parse_u8(value)),
                "rD" => data_raw.daily_calorie_intake_kcal = Some(TanitaParser::parse_u16(value)),
                "CS" => data_raw.checksum = TanitaParser::unquote(value),

                _ => {
                    println!("[DATA] Some extra key: {:?} and value: {:?}", key, value);
                    data_raw.extras.push((key.to_string(), value.to_string()));
                }
            }
            key_pointer = key_pointer + 2;
        }

        return data_raw;
    }
}

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

/// Clean profile info you actually use in the app.
#[derive(Debug)]
pub struct Profile {
    /// Date of birth (raw string from device; parse later if you adopt a date library).
    pub birth_date_dmy: Date,
    /// Gender as an enum (converted from raw `GE`).
    pub gender: Gender,
    /// Height in cm.
    pub height_cm: f32,
    /// Activity level (device code 1..N).
    pub activity_level_code: u8,
    /// Body/athlete mode code (device setting, raw).
    pub body_type_code: u8,
}

impl Profile {
    pub fn from_raw(raw: ProfRaw) -> Option<Profile> {
        let date = Date::from_string(&raw.birth_date_dmy)?;

        return Some(Profile {
            birth_date_dmy: date,
            body_type_code: raw.body_type_code,
            activity_level_code: raw.activity_level_code,
            height_cm: raw.height_cm,
            gender: Gender::from(raw.gender_code),
        });
    }
}

#[derive(Debug)]
pub struct Measurement {
    pub date_time: DateTime,

    // profile echo
    pub gender_code: Gender,
    pub age_years: u8,
    pub height_cm: f32,
    pub activity_level_code: u8,
    pub body_type_code: u8,

    // body metrics
    pub weight_kg: f32,
    pub bmi: f32,
    pub fat_percent: f32,

    // segmental fat
    pub fat_right_arm_pct: f32,
    pub fat_left_arm_pct: f32,
    pub fat_right_leg_pct: f32,
    pub fat_left_leg_pct: f32,
    pub fat_trunk_pct: f32,

    // optional extras
    pub muscle_percent: Option<f32>,
    pub muscle_right_arm_pct: Option<f32>,
    pub muscle_left_arm_pct: Option<f32>,
    pub muscle_right_leg_pct: Option<f32>,
    pub muscle_left_leg_pct: Option<f32>,
    pub muscle_trunk_pct: Option<f32>,

    pub bone_kg: Option<f32>,
    pub water_percent: Option<f32>,
    pub visceral_fat_rating: Option<u8>,
    pub metabolic_age_years: Option<u8>,
    pub daily_calorie_intake_kcal: Option<u16>,
}

impl Measurement {
    pub fn from_raw(raw: DataRaw) -> Option<Measurement> {
        let date_time = DateTime::from_string(&raw.date_dmy, &raw.time_hms)?;

        Some(Measurement {
            gender_code: Gender::from(raw.gender_code),
            date_time,
            height_cm: raw.height_cm,
            activity_level_code: raw.activity_level_code,
            body_type_code: raw.body_type_code,
            daily_calorie_intake_kcal: raw.daily_calorie_intake_kcal,
            metabolic_age_years: raw.metabolic_age_years,
            visceral_fat_rating: raw.visceral_fat_rating,
            water_percent: raw.water_percent,
            bone_kg: raw.bone_kg,
            muscle_trunk_pct: raw.muscle_trunk_pct,
            muscle_left_leg_pct: raw.muscle_left_leg_pct,
            muscle_right_leg_pct: raw.muscle_right_leg_pct,
            muscle_right_arm_pct: raw.muscle_right_arm_pct,
            muscle_left_arm_pct: raw.muscle_left_arm_pct,
            muscle_percent: raw.muscle_percent,
            fat_trunk_pct: raw.fat_trunk_pct,
            fat_left_leg_pct: raw.fat_left_leg_pct,
            fat_right_leg_pct: raw.fat_right_leg_pct,
            fat_left_arm_pct: raw.fat_left_arm_pct,
            fat_right_arm_pct: raw.fat_right_arm_pct,
            fat_percent: raw.fat_percent,
            bmi: raw.bmi,
            weight_kg: raw.weight_kg,
            age_years: raw.age_years,
        })
    }
}

#[derive(Debug)]
pub struct UserMeasurements {
    /// Pair index N (from filenames DATA{N}.CSV / PROF{N}.CSV).
    pub index: u32,
    /// Parsed, interpreted profile.
    pub profile: Profile,
    /// All measurements parsed from DATA{N}.CSV.
    pub measurements: Vec<Measurement>,
}

impl UserMeasurements {
    pub fn from_raw(raw: RawUserRecord) -> UserMeasurements {
        let profile = Profile::from_raw(raw.profile).unwrap();
        let mut measurements: Vec<Measurement> = Vec::with_capacity(raw.data.len());
        for data in raw.data {
            let m = Measurement::from_raw(data);
            match m {
                Some(m) => {
                    measurements.push(m);
                }
                None => {
                    println!("Measurement is missing")
                }
            }
        }
        UserMeasurements {
            index: raw.index,
            profile,
            measurements,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    PickFileOrFolder,
    PathPicked(Option<PathBuf>),
    TabSelected(usize),
}

#[derive(Default)]
struct UI {
    measurements: Vec<UserMeasurements>,
    selected_tab: usize,
}

impl UI {
    pub fn view(&self) -> Column<'_, Message> {
        let mut col = iced::widget::column![
            button("Choose [GRAPHV1] in a Tanita folder").on_press(Message::PickFileOrFolder),
        ]
        .padding(10)
        .spacing(10);

        if self.measurements.len() != 0 {
            let tab_titles = iced::widget::row((0..self.measurements.len()).map(|i| {
                button(text(format!("User {}", i + 1)))
                    .on_press(Message::TabSelected(i))
                    .into()
            }))
            .spacing(8);

            col = col.push(tab_titles);

            let u = &self.measurements[self.selected_tab];
            col = col.push(TableBuilder::heading(&u.profile));
            col = col.push(TableBuilder::body(&u.measurements));
        }

        col
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PickFileOrFolder => Task::perform(pick_folder(), Message::PathPicked),

            Message::PathPicked(path_buff) => {
                match path_buff {
                    Some(file) => {
                        let parser = TanitaParser { root_dir: file };
                        let raw = parser.get_raw_users_records();
                        let mut ui_ready_measurments: Vec<UserMeasurements> =
                            Vec::with_capacity(raw.len());

                        for e in raw {
                            ui_ready_measurments.push(UserMeasurements::from_raw(e));
                        }
                        self.measurements = ui_ready_measurments;
                    }
                    None => {
                        println!("path was not picked, how did u ended up here?");
                    }
                }
                Task::none()
            }

            Message::TabSelected(i) => {
                self.selected_tab = i;
                Task::none()
            }
        }
    }

    pub fn theme(_state: &UI) -> iced::Theme {
        Theme::CatppuccinMocha
    }

    pub fn title(_state: &UI) -> String {
        String::from("Here is the title of my app (todo: find a nice name)")
    }
}

pub struct TableBuilder {}
impl TableBuilder {
    pub fn text_w100<'a, T>(t: T) -> Text<'a>
    where
        T: text::IntoFragment<'a>,
    {
        text(t).width(Length::Fixed(100.0))
    }

    pub fn text_w50<'a, T>(t: T) -> Text<'a>
    where
        T: text::IntoFragment<'a>,
    {
        text(t).width(Length::Fixed(50.0))
    }

    pub fn option_into_string<T>(val: Option<T>) -> String
    where
        T: ToString,
    {
        match val {
            Some(v) => v.to_string(),
            None => "-".to_string(),
        }
    }

    pub fn heading(profile: &Profile) -> Column<'_, Message> {
        let title = iced::widget::row![
            TableBuilder::text_w100("Birht date"),
            TableBuilder::text_w100("Gender"),
            TableBuilder::text_w100("Height"),
            TableBuilder::text_w100("Activity level"),
            TableBuilder::text_w100("Body level"),
        ]
        .spacing(10);
        let content = iced::widget::row![
            TableBuilder::text_w100(profile.birth_date_dmy.to_srting()),
            TableBuilder::text_w100(profile.gender.to_string()),
            TableBuilder::text_w100(profile.height_cm.to_string()),
            TableBuilder::text_w100(profile.activity_level_code.to_string()),
            TableBuilder::text_w100(profile.body_type_code.to_string()),
        ]
        .spacing(10);

        iced::widget::column![title, content]
    }

    pub fn body(measurements: &Vec<Measurement>) -> Column<'_, Message> {
        let title = iced::widget::row![
            TableBuilder::text_w50("Date and time"),
            TableBuilder::text_w50("Age"),
            TableBuilder::text_w50("Activity level"),
            TableBuilder::text_w50("Body level"),
            TableBuilder::text_w50("Weight (kg)"),
            TableBuilder::text_w50("BMI"),
            TableBuilder::text_w50("Fat (%)"),
            TableBuilder::text_w50("Fat (%) torso"),
            TableBuilder::text_w50("Fat (%) r arm"),
            TableBuilder::text_w50("Fat (%) l arm"),
            TableBuilder::text_w50("Fat (%) r leg"),
            TableBuilder::text_w50("Fat (%) l leg"),
            TableBuilder::text_w50("Muscle (%)"),
            TableBuilder::text_w50("Muscle (%) torso"),
            TableBuilder::text_w50("Muscle (%) r arm"),
            TableBuilder::text_w50("Muscle (%) l arm"),
            TableBuilder::text_w50("Muscle (%) r leg"),
            TableBuilder::text_w50("Muscle (%) l leg"),
            TableBuilder::text_w50("Bones (kg)"),
            TableBuilder::text_w50("Water (%)"),
            TableBuilder::text_w50("Visceral fat raiting"),
            TableBuilder::text_w50("Metabolic age"),
            TableBuilder::text_w50("Daily calorie intake (kcal)"),
        ]
        .spacing(1);

        let mut col = iced::widget::column![];

        for measurement in measurements {
            let r = iced::widget::row![
                TableBuilder::text_w50(measurement.date_time.to_string()),
                TableBuilder::text_w50(measurement.age_years),
                TableBuilder::text_w50(measurement.activity_level_code),
                TableBuilder::text_w50(measurement.body_type_code),
                TableBuilder::text_w50(measurement.weight_kg),
                TableBuilder::text_w50(measurement.bmi),
                TableBuilder::text_w50(measurement.fat_percent),
                TableBuilder::text_w50(measurement.fat_trunk_pct),
                TableBuilder::text_w50(measurement.fat_right_arm_pct),
                TableBuilder::text_w50(measurement.fat_left_arm_pct),
                TableBuilder::text_w50(measurement.fat_right_leg_pct),
                TableBuilder::text_w50(measurement.fat_left_leg_pct),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.muscle_percent
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.muscle_trunk_pct
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.muscle_right_arm_pct
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.muscle_left_arm_pct
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.muscle_right_leg_pct
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.muscle_left_leg_pct
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(measurement.bone_kg)),
                TableBuilder::text_w50(TableBuilder::option_into_string(measurement.water_percent)),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.visceral_fat_rating
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.metabolic_age_years
                )),
                TableBuilder::text_w50(TableBuilder::option_into_string(
                    measurement.daily_calorie_intake_kcal
                )),
            ];
            col = col.push(r);
        }

        iced::widget::column![title, scrollable(col)]
    }
}

fn main() -> iced::Result {
    iced::application(UI::title, UI::update, UI::view)
        .theme(UI::theme)
        .run()
}
