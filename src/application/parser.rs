use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

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

#[derive(Debug)]
pub struct RawUserRecord {
    pub index: usize,
    pub profile: ProfRaw,
    pub data: Vec<DataRaw>,
}

pub struct TanitaParser {
    pub root_dir: PathBuf,
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

    fn get_index(&self, file_name: &str) -> Option<usize> {
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

    fn collect_files(&self, dir: &Path) -> BTreeMap<usize, PathBuf> {
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
    index: usize,
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
