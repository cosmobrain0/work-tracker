# Work Tracker
we need to define:
- a project
- a work slice
- payment

a project is:
- a group of work slices
- with a single description
- and a single work slice can be part of multiple projects
- and we can ask a work slice which projects it is a part of
- and has a unique ProjectID

a work slice is:
- a time span:
  - which can be "complete" (between two fixed dates)
  - or "incomplete" (between a fixed date and the present time, ongoing)
- with payment details
- which can be part of a project
- and has a unique WorkSliceID

Payment is:
- either hourly (a fixed rate per hour of working)
- or whole (a fixed payment for the entire work slice)

We then need a "state" and a "message-response" system:
- the "state" stores all projects, work-slices and their payments
- the state can be asked questions or given requests ("messages")
- the state will respond to questions with answers
- the state will respond to requests with success/failure

and then to figure out how to store "state" permanently, ideally in a database.

## Database

project:
- project_id (serial) (primary)
- name (varchar)
- description (varchar)

work_slice:
- work_id (serial) (primary)
- start (timestamp with time zone)
- end (timestamp with time zone OR null) (end is null means incomplete)
- payment (hourly bool, money int)
- project_id (int) (foreign project)

views: TODO


## So what are these Project and WorkSlice structs?

They can be used to locally store data about projects,
in some sort of cache or as the result of a query,
to be processed and displayed to the user.

So it makes sense to make them immutable,
with public getters and setters for *every* field.

they currently have associated functions for changing state,
which I want `State` to be responsible for.
They also have associated functions for getting related data,
which maybe ought to be pub(crate) for State to use,
and maybe they ought to get data from the database.

This might make it a little confusing,
as there will be some methods which take &self and return
locally stored data,
and there are others which just take a ProjectId and fetch
and return new data. Gonna need a good naming scheme.

Maybe these database functions should be in a new ProjectDatabase struct?

Ah, the solution is a trait, ofc:
```
// find a better name for this trait
trait Project {
  fn name(&self) -> Result<String, Box<dyn Error + Send + Sync>>;
  // and other getters, for any "fields" or "related data",
  // as both of those are the same thing in this context
  // since we don't know that an type which impls this trait
  // will even have these as fields

  fn start_work_now(&mut self) -> Result<(), Box<dyn Error + ...>>;
  // and other operations.
}

impl Project for LocalProject {
  fn name(&self) -> Result<...> {
    Ok(self.name.to_string())
  }
  // and other getters, which are always Ok().

  fn start_work_now(&mut self) -> Result<...> {
    self.start = IncompleteWorkSlice::new(...);
  }
  // and other operations, which mutate this object
}

impl Project for DatabaseProject {
  fn name(&self) -> Result<...> {
    // database query, not mutating self
  }
  // and other getters

  fn start_work_now(&mut self) -> Result<...> {
    // database query, not mutating self
  }
  // and other oeprations.
}
```

So `State` stores `Vec<DatabaseProject>` for now
and if we wanted caching we could also have a `Vec<LocalProject>`.
and these can be joined and operated on together,
as a `Vec<&mut dyn Project>` or `Vec<Box<dyn Project>>`.

actually, even better:
`State` is actually `State<T: Project>`.
And it stores `Vec<T>` for projects.
and so you can have a local `State<LocalProject>` or a non-local `State<DatabaseProject>`.

Just have to figure out how to make the other types work with this,
and how to implement caching.

Caching is inherently related to the database,
so `DatabaseProject` has a bunch of fields which are `(Option<T>, DateTime<Utc>)`
and if it's `(Some(x), not-long-before-now)` then the `x` is returned,
and otherwise a query is made and `not-long-before-now` becomes `Utc::now()`.

The `State` needs to be able to tell its owner a lot about projects.
It needs to be able to return a `Vec<CompleteWorkSlice>` and an `Option<IncompleteWorkSlice>`
to provide complete information.
So those need to be traits, like the `Project`, hiding two pub(crate) types for each trait.

Even better idea: instead of a `State<T>`, we have a `State::new(config: impl Config)`,
where `Config` has a method which creates a new project and returns it as an `impl ProjectTrait`,
and all the `State` methods work by interacting with `Project`s
