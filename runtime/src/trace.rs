use generated::maroon_assembler::{State, StepResult};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TraceEvent {
  pub state: State,
  pub result: StepResult,
}
