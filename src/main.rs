mod state;

use std::error::Error;
use std::io::ErrorKind;

use clap::{Parser, Subcommand};
use state::{Change, CompleteWorkSliceData, IncompleteWorkSliceData, Project, ProjectData, State};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new project
    Create {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        description: String,
    },
}

fn main() -> Result<(), ()> {
    let cli = Cli::parse();

    dotenvy::dotenv().expect("Couldn't load .env");
    let save_file_name = std::env::var("SAVE_FILE").expect("Couldn't load the SAVE_FILE variable");
    let initial_data = load_data(&save_file_name).expect("Failed to load data");

    let mut state = State::new(initial_data.clone(), move |changes, final_data| {
        save_data(&save_file_name, changes, final_data);
    })
    .expect("Failed to initialise State");

    match cli.command {
        Command::Create { name, description } => {
            let id = state.new_project(name, description);
            println!("Created project #{id}", id = unsafe { id.inner() });
        }
    }

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
