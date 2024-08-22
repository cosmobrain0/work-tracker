mod state;

use std::io::ErrorKind;
use std::{error::Error, str::FromStr};

use chrono::{DateTime, Duration, TimeDelta};
use clap::{Parser, Subcommand};
use state::{
    Change, CompleteWorkSlice, CompleteWorkSliceData, IncompleteWorkSlice, IncompleteWorkSliceData,
    Money, Project, ProjectData, ProjectId, State, WorkSlice, WorkSliceId,
};

use crate::state::{MoneyExact, Payment};

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
    All {
        #[arg(short, long)]
        verbose: bool,
    },
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
    dotenvy::dotenv().expect("Couldn't load .env");
    let save_file_name = std::env::var("SAVE_FILE").expect("Couldn't load the SAVE_FILE variable");
    let initial_data = load_data(&save_file_name).expect("Failed to load data");

    let cli = Cli::parse();

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
            ViewCommand::All { verbose } => {
                if verbose {
                    println!(
                        "{}",
                        state
                            .all_projects()
                            .map(format_project_verbose)
                            .reduce(|acc, e| format!("{acc}\n\n{e}"))
                            .unwrap_or_else(|| "No current projects.".to_string())
                    );
                } else {
                    println!(
                        "{}",
                        state
                            .all_projects()
                            .map(format_project_not_verbose)
                            .reduce(|acc, e| format!("{acc}\n\n{e}"))
                            .unwrap_or_else(|| "No current projects.".to_string())
                    );
                }
            }
            ViewCommand::Project {
                project_id,
                verbose,
            } => match state.project_from_id(unsafe { ProjectId::new(project_id) }) {
                Some(project) => {
                    println!(
                        "{}",
                        if verbose {
                            format_project_verbose(project)
                        } else {
                            format_project_not_verbose(project)
                        }
                    );
                }
                None => eprintln!("That project id ({project_id}) is invalid!"),
            },
            ViewCommand::Work { work_slice_id } => {
                match state.work_slice_from_id(unsafe { WorkSliceId::new(work_slice_id) }) {
                    Some(WorkSlice::Complete(complete)) => {
                        let project_id = state
                            .project_id_from_work_slice(unsafe { WorkSliceId::new(work_slice_id) })
                            .unwrap();
                        let payment = match complete.payment() {
                            Payment::Hourly(rate) => format!("{rate} / hour"),
                            Payment::Fixed(payment) => format!("fixed at {payment}"),
                        };
                        let start = complete.start().to_rfc2822();
                        let duration = format_duration(complete.duration());
                        let total_payment = complete.calculate_payment().as_pence();
                        let total_payment_pounds = (total_payment / 100.0).floor().to_string();
                        let total_payment_pence = (total_payment % 100.0).floor().to_string();
                        let completion = complete.completion().to_rfc2822();
                        println!(
                            "Completed work slice {work_slice_id} for project {project_id}: Payment is {payment} - started at {start}, lasting {duration}, ending at {completion} and earning {total_payment}",
                            project_id = unsafe { project_id.inner() },
                            total_payment = format!("£{total_payment_pounds} and {total_payment_pence} pence"),
                        );
                    }
                    Some(WorkSlice::Incomplete(incomplete)) => {
                        let project_id = state
                            .project_id_from_work_slice(unsafe { WorkSliceId::new(work_slice_id) })
                            .unwrap();
                        let payment = incomplete.payment();
                        let start = incomplete.start().to_rfc2822();
                        let duration = format_duration(incomplete.duration()).to_string();
                        let total_payment = incomplete.calculate_payment_so_far();
                        println!(
                            "Current work slice {work_slice_id} for project {project_id}: Payment is {payment} - started at {start}, lasting {duration} and earning {total_payment}",
                            project_id = unsafe { project_id.inner() },
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

fn format_complete_work_slice(work_slice: &CompleteWorkSlice) -> String {
    format!(
        "{id} - {start} - {end}, {payment}, {total_payment}",
        id = unsafe { work_slice.id().inner() },
        start = work_slice.start().to_rfc2822(),
        end = work_slice.completion().to_rfc2822(),
        payment = work_slice.payment(),
        total_payment = work_slice.calculate_payment(),
    )
}

fn format_incomplete_work_slice_verbose(work_slice: &IncompleteWorkSlice) -> String {
    format!(
        "{id} - started at {start}, {duration} ago, {payment} - {total_payment}",
        id = unsafe { work_slice.id().inner() },
        start = work_slice.start().to_rfc2822(),
        duration = format_duration(work_slice.duration()),
        payment = work_slice.payment(),
        total_payment = work_slice.calculate_payment_so_far(),
    )
}

fn format_incomplete_work_slice(work_slice: &IncompleteWorkSlice) -> String {
    format!(
        "{id} - started {duration} ago",
        id = unsafe { work_slice.id().inner() },
        duration = format_duration(work_slice.duration())
    )
}

fn format_project_not_verbose(project: &Project) -> String {
    let completed_work: Vec<_> = project.complete_work_slices().collect();
    let incomplete_work: Option<&IncompleteWorkSlice> = project.current_work_slice();

    let completed_work_string = {
        let number = completed_work.len();
        let list = completed_work
            .iter()
            .map(|x| x.id())
            .map(|x| unsafe { x.inner().to_string() })
            .reduce(|acc, e| format!("{acc}, {e}"))
            .unwrap_or_default();
        format!("- completed work: {number} [{list}]")
    };
    let incomplete_work_string = match &incomplete_work {
        Some(work) => format!(
            "{id} - started {duration} ago",
            id = unsafe { work.id().inner() },
            duration = format_duration(work.duration())
        ),
        None => "not working".to_string(),
    };

    let total_duration = format_duration(
        completed_work
            .iter()
            .map(|x| x.duration())
            .sum::<TimeDelta>()
            + incomplete_work
                .map(|x| x.duration())
                .unwrap_or(TimeDelta::zero()),
    );
    let total_payment = completed_work
        .iter()
        .map(|x| x.calculate_payment())
        .sum::<MoneyExact>()
        + incomplete_work
            .map(|x| x.calculate_payment_so_far())
            .unwrap_or(MoneyExact::new(0.0).unwrap());
    let top_line = format!(
        "Project {id}: {name} ({total_duration}, {total_payment})",
        id = unsafe { project.id().inner() },
        name = project.name(),
    );
    format!("{top_line}\n{completed_work_string}\n- current work: {incomplete_work_string}")
}

fn format_project_verbose(project: &Project) -> String {
    let completed_work: Vec<_> = project.complete_work_slices().collect();
    let incomplete_work: Option<&IncompleteWorkSlice> = project.current_work_slice();

    let completed_work_string = {
        let data = completed_work
            .iter()
            .map(|x| format!("  {}", format_complete_work_slice(x)))
            .reduce(|acc, e| format!("{acc}\n{e}"));
        match data {
            Some(x) => format!("- completed work: {len}\n{x}", len = completed_work.len()),
            None => "- no completed work".to_string(),
        }
    };
    let incomplete_work_string = match &incomplete_work {
        Some(work) => format!(
            "{id} - started at {start}, {duration} ago, {payment} - {total_payment}",
            id = unsafe { work.id().inner() },
            start = work.start().to_rfc2822(),
            duration = format_duration(work.duration()),
            payment = work.payment(),
            total_payment = work.calculate_payment_so_far()
        ),
        None => "not owrking".to_string(),
    };

    let total_duration = format_duration(
        completed_work
            .iter()
            .map(|x| x.duration())
            .sum::<TimeDelta>()
            + incomplete_work
                .map(|x| x.duration())
                .unwrap_or(TimeDelta::zero()),
    );
    let total_payment = completed_work
        .iter()
        .map(|x| x.calculate_payment())
        .sum::<MoneyExact>()
        + incomplete_work
            .map(|x| x.calculate_payment_so_far())
            .unwrap_or(MoneyExact::new(0.0).unwrap());
    let top_line = format!(
        "Project {id}: {name} ({total_duration}, {total_payment})",
        id = unsafe { project.id().inner() },
        name = project.name(),
    );
    let description = project.description();
    format!("{top_line}\n{description}\n{completed_work_string}\n- current work: {incomplete_work_string}")
}

fn format_duration(duration: Duration) -> String {
    let hours = duration.num_seconds() / (60 * 60);
    let minutes = (duration.num_seconds() / 60) % 60;
    let seconds = duration.num_seconds() % 60;
    let hours = if hours == 0 {
        None
    } else {
        Some(format!("{hours} hours"))
    };
    let minutes = if minutes == 0 {
        None
    } else {
        Some(format!("{minutes} minutes"))
    };
    let seconds = if seconds == 0 {
        None
    } else {
        Some(format!("{seconds} seconds"))
    };
    match (hours, minutes, seconds) {
        (None, None, None) => "0 seconds".to_string(),
        (None, None, Some(s)) => s,
        (None, Some(m), None) => m,
        (None, Some(m), Some(s)) => format!("{m} and {s}"),
        (Some(h), None, None) => h,
        (Some(h), None, Some(s)) => format!("{h} and {s}"),
        (Some(h), Some(m), None) => format!("{h} and {m}"),
        (Some(h), Some(m), Some(s)) => format!("{h}, {m} and {s}"),
    }
}
