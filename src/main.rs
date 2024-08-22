mod state;

use std::io::ErrorKind;
use std::{error::Error, str::FromStr};

use chrono::{DateTime, Utc};
use state::{
    Change, CompleteWorkSliceData, IncompleteWorkSliceData, Money, Payment, Project, ProjectData,
    State,
};

fn main() -> Result<(), ()> {
    dotenvy::dotenv().expect("Couldn't load .env");
    let save_file_name = std::env::var("SAVE_FILE").expect("Couldn't load the SAVE_FILE variable");
    let initial_data = load_data(&save_file_name).expect("Failed to load data");

    let mut state = State::new(initial_data.clone(), move |changes, final_data| {
        save_data(&save_file_name, changes, final_data);
    })
    .expect("Failed to initialise State");

    let project = state.new_project(
        "Example Project".to_string(),
        "This is an example project with a complete work slice and an incomplete one!".to_string(),
    );
    state
        .start_work(
            project,
            Payment::Hourly(Money::new(800)),
            DateTime::from_str("2024-08-21T08:00:00Z").unwrap(),
        )
        .expect("Failed to start work slice 1");
    state
        .end_work(project, DateTime::from_str("2024-08-21T10:00:00Z").unwrap())
        .expect("Failed to start work slice 3");
    state
        .start_work(
            project,
            Payment::Fixed(Money::new(2000)),
            DateTime::from_str("2024-08-21T14:30:00Z").unwrap(),
        )
        .expect("Failed to start work slice 2");
    state
        .end_work(project, Utc::now())
        .expect("Failed to end work slice 2");

    Ok(())
}

fn load_data(file_name: &str) -> Result<Vec<ProjectData>, Box<dyn Error>> {
    let file = match std::fs::read_to_string(file_name) {
        Ok(x) => x,
        Err(x) => match x.kind() {
            ErrorKind::NotFound => {
                std::fs::write(file_name, "[]")?;
                "[]".to_string()
            }
            _ => {
                return Err(Box::new(x));
            }
        },
    };
    let data: Vec<ProjectData> = serde_json::from_str(&file)?;
    Ok(data)
}
fn save_data(file_name: &str, _changes: Vec<Change>, final_data: Vec<&Project>) {
    std::fs::write(
        file_name,
        serde_json::to_string(
            &final_data
                .into_iter()
                .map(|x| ProjectData {
                    name: x.name().to_string(),
                    description: x.description().to_string(),
                    work_slices: x
                        .complete_work_slices()
                        .map(|x| CompleteWorkSliceData {
                            start: x.start(),
                            end: x.completion(),
                            payment: x.payment(),
                            id: unsafe { x.id().inner() },
                        })
                        .collect(),
                    current_slice: x.current_work_slice().map(|x| IncompleteWorkSliceData {
                        start: x.start(),
                        payment: x.payment(),
                        id: unsafe { x.id().inner() },
                    }),
                    id: unsafe { x.id().inner() },
                })
                .collect::<Vec<_>>(),
        )
        .expect("Failed to serialize data"),
    )
    .expect("Failed to save data");
}
