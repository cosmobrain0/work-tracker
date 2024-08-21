macro_rules! derive_debug_error {
    ($type: ty) => {
        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{:#?}", self)
            }
        }
        impl std::error::Error for $type {}
    };

    ($($type:ty)*) => {
        $(derive_debug_error!{$type})*
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteWorkError {
    NoWorkToComplete,
    EndTimeTooEarly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStartNowError {
    AlreadyStarted,
    InvalidProjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkEndNowError {
    NoCurrentWork,
    InvalidProjectId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotFoundError {
    ProjectNotFound,
    WorkSliceNotFound,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkSliceNotFoundError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkStartError {
    AlreadyStarted,
    InvalidProjectId,
    InvalidStartTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkEndError {
    EndTimeTooEarly,
    NoWorkToComplete,
    InvalidProjectId,
}
impl From<CompleteWorkError> for WorkEndError {
    fn from(value: CompleteWorkError) -> Self {
        match value {
            CompleteWorkError::NoWorkToComplete => Self::NoWorkToComplete,
            CompleteWorkError::EndTimeTooEarly => Self::EndTimeTooEarly,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InvalidProjectId;

derive_debug_error! {
    CompleteWorkError
    WorkStartNowError
    WorkEndNowError
    NotFoundError
    WorkSliceNotFoundError
    WorkStartError
    InvalidProjectId
}
