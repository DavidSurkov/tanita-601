use std::{path::PathBuf, usize};

use iced::{
    Length, Task, Theme,
    widget::{Column, Text, button, scrollable, text},
};

use rfd::AsyncFileDialog;

mod general_data_structs;
use general_data_structs::{Date, DateTime, Gender};

mod parser;
use parser::{DataRaw, ProfRaw, RawUserRecord, TanitaParser};

async fn pick_folder() -> Option<PathBuf> {
    let file_handle = AsyncFileDialog::new()
        //TODO: set /home folder as root
        .set_title("Pick [GRAPHV1] folder isnide TANITA folder")
        .pick_folder()
        .await?;

    let path: PathBuf = file_handle.into();

    return Some(path);
}

/// Clean profile info you actually use in the app.
#[derive(Debug)]
struct Profile {
    /// Date of birth (raw string from device; parse later if you adopt a date library).
    birth_date_dmy: Date,
    /// Gender as an enum (converted from raw `GE`).
    gender: Gender,
    /// Height in cm.
    height_cm: f32,
    /// Activity level (device code 1..N).
    activity_level_code: u8,
    /// Body/athlete mode code (device setting, raw).
    body_type_code: u8,
}

impl Profile {
    fn from_raw(raw: ProfRaw) -> Option<Profile> {
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
struct Measurement {
    date_time: DateTime,

    // profile echo
    //  gender_code: Gender,
    age_years: u8,
    //  height_cm: f32,
    activity_level_code: u8,
    body_type_code: u8,

    // body metrics
    weight_kg: f32,
    bmi: f32,
    fat_percent: f32,

    // segmental fat
    fat_right_arm_pct: f32,
    fat_left_arm_pct: f32,
    fat_right_leg_pct: f32,
    fat_left_leg_pct: f32,
    fat_trunk_pct: f32,

    // optional extras
    muscle_percent: Option<f32>,
    muscle_right_arm_pct: Option<f32>,
    muscle_left_arm_pct: Option<f32>,
    muscle_right_leg_pct: Option<f32>,
    muscle_left_leg_pct: Option<f32>,
    muscle_trunk_pct: Option<f32>,

    bone_kg: Option<f32>,
    water_percent: Option<f32>,
    visceral_fat_rating: Option<u8>,
    metabolic_age_years: Option<u8>,
    daily_calorie_intake_kcal: Option<u16>,
}

impl Measurement {
    fn from_raw(raw: DataRaw) -> Option<Measurement> {
        let date_time = DateTime::from_string(&raw.date_dmy, &raw.time_hms)?;

        Some(Measurement {
            // gender_code: Gender::from(raw.gender_code),
            date_time,
            // height_cm: raw.height_cm,
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
struct UserMeasurements {
    /// Pair index N (from filenames DATA{N}.CSV / PROF{N}.CSV).
    index: usize,
    /// Parsed, interpreted profile.
    profile: Profile,
    /// All measurements parsed from DATA{N}.CSV.
    measurements: Vec<Measurement>,
}

impl UserMeasurements {
    fn from_raw(raw: RawUserRecord) -> UserMeasurements {
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
enum Message {
    PickFileOrFolder,
    PathPicked(Option<PathBuf>),
    TabSelected(usize),
}

#[derive(Default)]
pub struct Application {
    measurements: Vec<UserMeasurements>,
    selected_tab: usize,
}

impl Application {
    fn view(&self) -> Column<'_, Message> {
        let mut col = iced::widget::column![
            button("Choose [GRAPHV1] in a Tanita folder").on_press(Message::PickFileOrFolder),
        ]
        .padding(10)
        .spacing(10);

        if self.measurements.len() != 0 {
            let mut tab_titles = iced::widget::row![].spacing(8);
            for user_mes in &self.measurements {
                tab_titles = tab_titles.push(
                    button(text(format!("User {}", user_mes.index + 1)))
                        .on_press(Message::TabSelected(user_mes.index)),
                );
            }

            col = col.push(tab_titles);

            let u = &self.measurements[self.selected_tab];
            col = col.push(TableBuilder::heading(&u.profile));
            col = col.push(TableBuilder::body(&u.measurements));
        }

        col
    }

    fn update(&mut self, message: Message) -> Task<Message> {
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

    fn theme(_state: &Application) -> iced::Theme {
        Theme::CatppuccinMocha
    }

    fn title(_state: &Application) -> String {
        String::from("Here is the title of my app (todo: find a nice name)")
    }

    pub fn run() -> iced::Result {
        iced::application(Self::title, Self::update, Self::view)
            .theme(Self::theme)
            .run()
    }
}

struct TableBuilder {}
impl TableBuilder {
    fn text_w100<'a, T>(t: T) -> Text<'a>
    where
        T: text::IntoFragment<'a>,
    {
        text(t).width(Length::Fixed(100.0))
    }

    fn text_w50<'a, T>(t: T) -> Text<'a>
    where
        T: text::IntoFragment<'a>,
    {
        text(t).width(Length::Fixed(50.0))
    }

    fn option_into_string<T>(val: Option<T>) -> String
    where
        T: ToString,
    {
        match val {
            Some(v) => v.to_string(),
            None => "-".to_string(),
        }
    }

    fn heading(profile: &Profile) -> Column<'_, Message> {
        let title = iced::widget::row![
            Self::text_w100("Birht date"),
            Self::text_w100("Gender"),
            Self::text_w100("Height"),
            Self::text_w100("Activity level"),
            Self::text_w100("Body level"),
        ]
        .spacing(10);
        let content = iced::widget::row![
            Self::text_w100(profile.birth_date_dmy.to_srting()),
            Self::text_w100(profile.gender.to_string()),
            Self::text_w100(profile.height_cm.to_string()),
            Self::text_w100(profile.activity_level_code.to_string()),
            Self::text_w100(profile.body_type_code.to_string()),
        ]
        .spacing(10);

        iced::widget::column![title, content]
    }

    fn body(measurements: &Vec<Measurement>) -> Column<'_, Message> {
        let title = iced::widget::row![
            Self::text_w50("Date and time"),
            Self::text_w50("Age"),
            Self::text_w50("Activity level"),
            Self::text_w50("Body level"),
            Self::text_w50("Weight (kg)"),
            Self::text_w50("BMI"),
            Self::text_w50("Fat (%)"),
            Self::text_w50("Fat (%) torso"),
            Self::text_w50("Fat (%) r arm"),
            Self::text_w50("Fat (%) l arm"),
            Self::text_w50("Fat (%) r leg"),
            Self::text_w50("Fat (%) l leg"),
            Self::text_w50("Muscle (%)"),
            Self::text_w50("Muscle (%) torso"),
            Self::text_w50("Muscle (%) r arm"),
            Self::text_w50("Muscle (%) l arm"),
            Self::text_w50("Muscle (%) r leg"),
            Self::text_w50("Muscle (%) l leg"),
            Self::text_w50("Bones (kg)"),
            Self::text_w50("Water (%)"),
            Self::text_w50("Visceral fat raiting"),
            Self::text_w50("Metabolic age"),
            Self::text_w50("Daily calorie intake (kcal)"),
        ]
        .spacing(1);

        let mut col = iced::widget::column![];

        for measurement in measurements {
            let r = iced::widget::row![
                Self::text_w50(measurement.date_time.to_string()),
                Self::text_w50(measurement.age_years),
                Self::text_w50(measurement.activity_level_code),
                Self::text_w50(measurement.body_type_code),
                Self::text_w50(measurement.weight_kg),
                Self::text_w50(measurement.bmi),
                Self::text_w50(measurement.fat_percent),
                Self::text_w50(measurement.fat_trunk_pct),
                Self::text_w50(measurement.fat_right_arm_pct),
                Self::text_w50(measurement.fat_left_arm_pct),
                Self::text_w50(measurement.fat_right_leg_pct),
                Self::text_w50(measurement.fat_left_leg_pct),
                Self::text_w50(Self::option_into_string(measurement.muscle_percent)),
                Self::text_w50(Self::option_into_string(measurement.muscle_trunk_pct)),
                Self::text_w50(Self::option_into_string(measurement.muscle_right_arm_pct)),
                Self::text_w50(Self::option_into_string(measurement.muscle_left_arm_pct)),
                Self::text_w50(Self::option_into_string(measurement.muscle_right_leg_pct)),
                Self::text_w50(Self::option_into_string(measurement.muscle_left_leg_pct)),
                Self::text_w50(Self::option_into_string(measurement.bone_kg)),
                Self::text_w50(Self::option_into_string(measurement.water_percent)),
                Self::text_w50(Self::option_into_string(measurement.visceral_fat_rating)),
                Self::text_w50(Self::option_into_string(measurement.metabolic_age_years)),
                Self::text_w50(Self::option_into_string(
                    measurement.daily_calorie_intake_kcal
                )),
            ];
            col = col.push(r);
        }

        iced::widget::column![title, scrollable(col)]
    }
}
