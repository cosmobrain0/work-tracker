# Work Tracker
This is currently a CLI tool, but I intend to refactor it to make a crate which can be turned into a CLI tool or a website or anything else.

This is a work tracker - you can use it to record the time you spend working on different projects and calculate how much money you are owed:

## Model
Everything is split into projects, which have:
- "complete work slices" - periods of work which have been completed. These have a start date, and end date, and a payment method (either a fixed payment or an hourly payment)
- an optional "incomplete work slice" - a period of work which is currently ongoing for this project. This has a start date and a payment method (either a fixed payment or an hourly payment)

## CLI Tool Usage
clone this repo and build it (it's stable Rust), then add the executable file to your PATH, and run it. Use `work-tracker --help` to see what the commands are.

## Crate Usage
Right now, I'm working on extracting everything in the `State` folder into a separate crate.  
The `State` type stores all of your data, and has a pretty self-explanatory public API for modifying the project data.  
The only parts which really require explanation are `State::new(initial_data: Vec<ProjectData>, commit_on_drop: impl Fn(Vec<Change>, Vec<&Project>) + 'static)` and `State::handle_changes(&mut self) -> Vec<Change>`:
- the "initial state" parameter to `State::new` is all of your saved project data. You're responsible for loading this from a file, or a database, or whatever. To start with nothing, just pass an empty vector (`Vec::new()` or `vec![]`) 
- the `State::handle_changes` function returns a list of changes made to the state since the previous call to `State::handle_changes`, or since this `State` was constructed (if `handle_changes` hasn't been called). You can use this to update your permanent storage whenever you like.
- the "commit on drop" function is called when this `State` falls out of scope (e.g. at the end of the program). This can be used to save the data stored in `State` to a local file or database. It takes a list of changes since the last call to `handle_changes` (or since this `State` was constructed, if `handle_changes` was never called) which you can use to make incremental updates, and it also takes all of the project data in a `Vec<&Project>`, if you want to just overwrite your storage completely.
