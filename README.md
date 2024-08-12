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

