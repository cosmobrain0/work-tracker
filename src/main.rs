mod state;

use std::error::Error;
use std::io::ErrorKind;

use clap::{Parser, Subcommand};
use state::{
    Change, CompleteWorkSliceData, IncompleteWorkSliceData, Project, ProjectData, ProjectId, State,
    WorkSlice, WorkSliceId,
};

use crate::state::Payment;

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
    Delete {
        #[command(subcommand)]
        command: DeleteCommand,
    },
    View {
        #[command(subcommand)]
        command: ViewCommand,
    },
}

#[derive(Subcommand)]
enum DeleteCommand {
    Project {
        project_id: u64,
    },
    Work {
        work_slice_id: u64,
        #[arg(short, long = "project")]
        project_id: u64,
    },
}

#[derive(Subcommand)]
enum ViewCommand {
    All,
    Project {
        project_id: u64,
        #[arg(short, long)]
        verbose: bool,
    },
    Work {
        work_slice_id: u64,
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
            println!("Created project {id}", id = unsafe { id.inner() });
        }
        Command::Delete { command } => match command {
            DeleteCommand::Project { project_id } => {
                if state.delete_project(unsafe { ProjectId::new(project_id) }) {
                    println!("Deleted project {project_id}");
                } else {
                    eprintln!("Can't delete project {project_id} as it doesn't exist!");
                }
            }
            // FIXME: this is untested because there's no way to start work rn
            DeleteCommand::Work {
                work_slice_id,
                project_id,
            } => unsafe {
                match state.project_id_from_work_slice(WorkSliceId::new(work_slice_id)).map(|x| x.inner()) {
                    Some(x) if x == project_id => {
                        state.delete_work_slice_from_project(ProjectId::new(project_id), WorkSliceId::new(work_slice_id));
                    },
                    Some(other_project_id) => eprintln!("That work slice ID ({work_slice_id}) belongs to another project ({other_project_id})!"),
                    None => {
                        eprintln!("That work slice ID ({work_slice_id}) is invalid!");
                    },
                }
                state.delete_work_slice_from_project(
                    ProjectId::new(project_id),
                    WorkSliceId::new(work_slice_id),
                );
            },
        },
        Command::View { command } => match command {
            ViewCommand::All => todo!(),
            ViewCommand::Project {
                project_id,
                verbose,
            } => todo!(),
            ViewCommand::Work { work_slice_id } => {
                match state.work_slice_from_id(unsafe { WorkSliceId::new(work_slice_id) }) {
                    Some(WorkSlice::Complete(complete)) => todo!(),
                    Some(WorkSlice::Incomplete(incomplete)) => {
                        let project_id = state
                            .project_id_from_work_slice(unsafe { WorkSliceId::new(work_slice_id) })
                            .unwrap();
                        let payment = match incomplete.payment() {
                            Payment::Hourly(rate) => format!("{rate} / hour"),
                            Payment::Fixed(payment) => format!("fixed at {payment}"),
                        };
                        let start = incomplete.start().to_rfc2822();
                        let duration = incomplete.duration().to_string();
                        let total_payment = incomplete.calculate_payment_so_far().as_pence();
                        let total_payment_pounds = (total_payment / 100.0).floor().to_string();
                        let total_payment_pence = (total_payment % 100.0).floor().to_string();
                        println!(
                            "Current work slice {work_slice_id} for project {project_id}: Payment is {payment} - started at {start}, lasting {duration} and earning {total_payment}",
                            project_id = unsafe { project_id.inner() },
                            total_payment = format!("£{total_payment_pounds} and {total_payment_pence} pence"),
                        );
                    }
                    None => eprintln!("That work slice id ({work_slice_id}) is invalid!"),
                }
            }
        },
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
