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
