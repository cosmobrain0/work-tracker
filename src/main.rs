mod state;

use std::io::ErrorKind;
use std::{error::Error, str::FromStr};

use chrono::{DateTime, Duration, TimeDelta, Utc};
use clap::{Parser, Subcommand};
use state::{
    Change, CompleteWorkSlice, CompleteWorkSliceData, IncompleteWorkSlice, IncompleteWorkSliceData,
    Money, Project, ProjectData, ProjectId, State, WorkEndError, WorkSlice, WorkSliceId,
    WorkStartError,
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
    List {
        #[command(subcommand)]
        command: ListCommand,
    },
    Start {
        project: u64,
        #[arg(short, long)]
        time: Option<DateTime<Utc>>,
        #[arg(short, long)]
        payment_fixed: bool,
        #[arg(short, long)]
        payment: u32,
    },
    Complete {
        project: u64,
        #[arg(short, long)]
        time: Option<DateTime<Utc>>,
    },
    DeleteWork {
        #[arg(short, long)]
        project: u64,
        work_slice: u64,
    },
}

#[derive(Subcommand)]
enum ListCommand {
    Projects {
        #[arg(short, long)]
        verbose: bool,
    },
    WorkSlices {
        project: Option<u64>,
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
            create_project(&mut state, name, description);
        }
        Command::Delete { command } => match command {
            DeleteCommand::Project { project_id } => {
                delete_project(&mut state, project_id);
            }
            DeleteCommand::Work {
                work_slice_id,
                project_id,
            } => unsafe {
                delete_work_slice(&mut state, work_slice_id, project_id);
            },
        },
        Command::View { command } => match command {
            ViewCommand::All { verbose } => {
                view_all_projects(verbose, &state);
            }
            ViewCommand::Project {
                project_id,
                verbose,
            } => view_project(&state, project_id, verbose),
            ViewCommand::Work { work_slice_id } => {
                view_work_slice(state, work_slice_id);
            }
        },
        Command::List { command } => match command {
            ListCommand::Projects { verbose } => view_all_projects(verbose, &mut state),
            ListCommand::WorkSlices { project: None } => {
                println!(
                    "{}",
                    state
                        .all_projects()
                        .map(|x| x.complete_work_slices())
                        .flatten()
                        .map(|x| view_single_complete_work_slice(&state, x))
                        .reduce(|acc, e| format!("{acc}\n{e}"))
                        .unwrap_or_else(|| String::from("No recorded work."))
                );
                println!(
                    "{}",
                    state
                        .all_projects()
                        .filter_map(|x| x.current_work_slice())
                        .map(|x| view_single_incomplete_work_slice(&state, x))
                        .reduce(|acc, e| format!("{acc}\n{e}"))
                        .unwrap_or_else(|| String::from("No ongoing work."))
                );
            }
            ListCommand::WorkSlices {
                project: Some(project_id),
            } => {
                let project = state.project_from_id(unsafe { ProjectId::new(project_id) });
                match project {
                    None => eprintln!("That project ID ({project_id}) is invalid!"),
                    Some(project) => {
                        println!(
                            "{}",
                            project
                                .complete_work_slices()
                                .map(|x| view_single_complete_work_slice(&state, x))
                                .reduce(|acc, e| format!("{acc}\n{e}"))
                                .unwrap_or_else(|| String::from(
                                    "No recorded work for project {project_id}."
                                ))
                        );
                        println!(
                            "{}",
                            project
                                .current_work_slice()
                                .map(|x| view_single_incomplete_work_slice(&state, x))
                                .unwrap_or_else(|| String::from(
                                    "No ongoing work for project {project_id}."
                                ))
                        );
                    }
                }
            }
        },
        Command::Start {
            project,
            time,
            payment_fixed,
            payment,
        } => {
            let payment = if payment_fixed {
                Payment::Fixed(Money::new(payment))
            } else {
                Payment::Hourly(Money::new(payment))
            };
            let time = time.unwrap_or_else(Utc::now);
            match state.start_work(unsafe { ProjectId::new(project) }, payment, time) {
                Ok(()) => println!(
                    "Started work for project {project} at time {time}.",
                    time = time.to_rfc2822()
                ),
                Err(err) => match err {
                    WorkStartError::AlreadyStarted => eprintln!(
                        "Can't start work for project {project} as some work is already ongoing!"
                    ),
                    WorkStartError::InvalidProjectId => {
                        eprintln!("That project ID ({project}) is invalid!")
                    }
                    WorkStartError::InvalidStartTime => {
                        eprintln!("The start time for work can't be in the future!")
                    }
                },
            }
        }
        Command::Complete { project, time } => {
            let id = unsafe { ProjectId::new(project) };
            let time = time.unwrap_or_else(Utc::now);
            match state.end_work(id, time) {
                Ok(()) => {
                    println!("Successfully worked work for project {project} as complete!");
                }
                Err(err) => match err {
                    WorkEndError::EndTimeTooEarly => {
                        eprintln!("The end time of work must be after the start time!")
                    }
                    WorkEndError::NoWorkToComplete => {
                        eprintln!("There is no ongoing work to mark as complete!")
                    }
                    WorkEndError::InvalidProjectId => {
                        eprintln!("That project ID ({project}) is invalid!")
                    }
                },
            }
        }
        Command::DeleteWork {
            project,
            work_slice,
        } => {
            let (project_id, work_slice_id) =
                unsafe { (ProjectId::new(project), WorkSliceId::new(project)) };
            let data = match state.project_from_id(project_id) {
                Some(project_data) => {
                    if let Some(work_slice) = project_data.work_slice_from_id(work_slice_id) {
                        match work_slice {
                            WorkSlice::Complete(complete) => {
                                Some(format_complete_work_slice(complete))
                            }
                            WorkSlice::Incomplete(incomplete) => {
                                Some(format_incomplete_work_slice_verbose(incomplete))
                            }
                        }
                    } else {
                        eprintln!("That work slice ID ({work_slice}) either doesn't exist, or isn't a part of the project {project}. It may have been deleted.");
                        None
                    }
                }
                None => {
                    eprintln!("That project ID ({project}) is invalid!");
                    None
                }
            };
            state.delete_work_slice_from_project(project_id, work_slice_id);
            if let Some(data) = data {
                println!("{data}");
            }
        }
    }

    Ok(())
}

fn view_work_slice(state: State, work_slice_id: u64) {
    match state.work_slice_from_id(unsafe { WorkSliceId::new(work_slice_id) }) {
        Some(WorkSlice::Complete(complete)) => {
            println!("{}", view_single_complete_work_slice(&state, complete));
        }
        Some(WorkSlice::Incomplete(incomplete)) => {
            println!("{}", view_single_incomplete_work_slice(&state, incomplete));
        }
        None => eprintln!("That work slice id ({work_slice_id}) is invalid!"),
    }
}

fn view_single_incomplete_work_slice(state: &State, incomplete: &IncompleteWorkSlice) -> String {
    let project_id = state.project_id_from_work_slice(incomplete.id()).unwrap();
    let payment = incomplete.payment();
    let start = incomplete.start().to_rfc2822();
    let duration = format_duration(incomplete.duration()).to_string();
    let total_payment = incomplete.calculate_payment_so_far();
    format!(
        "Current work slice {work_slice_id} for project {project_id}: Payment is {payment} - started at {start}, lasting {duration} and earning {total_payment}",
        project_id = unsafe { project_id.inner() },
        work_slice_id = unsafe { incomplete.id().inner() },
    )
}

fn view_single_complete_work_slice(state: &State, complete: &CompleteWorkSlice) -> String {
    let project_id = state.project_id_from_work_slice(complete.id()).unwrap();
    let payment = match complete.payment() {
        Payment::Hourly(rate) => format!("{rate} / hour"),
        Payment::Fixed(payment) => format!("fixed at {payment}"),
    };
    let start = complete.start().to_rfc2822();
    let duration = format_duration(complete.duration());
    let total_payment = complete.calculate_payment().as_pence();
    let completion = complete.completion().to_rfc2822();
    format!(
        "Completed work slice {work_slice_id} for project {project_id}: Payment is {payment} - started at {start}, lasting {duration}, ending at {completion} and earning {total_payment}",
        project_id = unsafe { project_id.inner() },
        work_slice_id = unsafe { complete.id().inner() },
    )
}

fn view_project(state: &State, project_id: u64, verbose: bool) {
    match state.project_from_id(unsafe { ProjectId::new(project_id) }) {
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
    }
}

fn view_all_projects(verbose: bool, state: &State) {
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

unsafe fn delete_work_slice(state: &mut State, work_slice_id: u64, project_id: u64) {
    match state
        .project_id_from_work_slice(WorkSliceId::new(work_slice_id))
        .map(|x| x.inner())
    {
        Some(x) if x == project_id => {
            state.delete_work_slice_from_project(
                ProjectId::new(project_id),
                WorkSliceId::new(work_slice_id),
            );
        }
        Some(other_project_id) => eprintln!(
            "That work slice ID ({work_slice_id}) belongs to another project ({other_project_id})!"
        ),
        None => {
            eprintln!("That work slice ID ({work_slice_id}) is invalid!");
        }
    }
    state.delete_work_slice_from_project(
        ProjectId::new(project_id),
        WorkSliceId::new(work_slice_id),
    );
}

fn delete_project(state: &mut State, project_id: u64) {
    if state.delete_project(unsafe { ProjectId::new(project_id) }) {
        println!("Deleted project {project_id}");
    } else {
        eprintln!("Can't delete project {project_id} as it doesn't exist!");
    }
}

fn create_project(state: &mut State, name: String, description: String) {
    let id = state.new_project(name, description);
    println!("Created project {id}", id = unsafe { id.inner() });
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
