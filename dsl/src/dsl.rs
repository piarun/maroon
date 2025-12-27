use std::any;

// Minimal function-like macro: `fiber!("name", { /* items */ })` or `fiber!("name" { /* items */ })`
// For now, it simply expands to the provided items so the code remains type-checkable.
// Later, it will also construct IR alongside preserving the items.
#[macro_export]
macro_rules! fiber {
  ($name:literal, { $($body:item)* }) => {
    const _: () = {
      $($body)*
    };
  };
}

// begin maroon `library` functions section

struct MrnQueue {}

impl MrnQueue {
  fn send(&mut self) {}
}

struct MrnFuture {}

impl MrnFuture {
  fn resolve(&mut self) {}
}

#[derive(Debug)]
enum Error {}
enum MrnCreateAsyncPrimitives {
  Queue { name: String, public: bool },
  Future,
}

// Helper: expand a single request into a typed expression
macro_rules! __mrn_create_primitive_expr {
  ( MrnCreateAsyncPrimitives::Queue { $($rest:tt)* } ) => {
    ::core::result::Result::<MrnQueue, Error>::Ok(MrnQueue {})
  };
  ( MrnCreateAsyncPrimitives::Future ) => {
    ::core::result::Result::<MrnFuture, Error>::Ok(MrnFuture {})
  };
}

macro_rules! mrn_create_primitives {
  ( vec![ $($t:tt)* ] ) => {
    mrn_create_primitives!(@as_tuple [] $($t)* )
  };
  (@as_tuple [ $($out:tt)* ] ) => { ( $($out)* ) };
  (@as_tuple [ $($out:tt)* ] MrnCreateAsyncPrimitives::Queue { $($q:tt)* } $(, $($rest:tt)* )? ) => {
    mrn_create_primitives!(
      @as_tuple [ $($out)* ::core::result::Result::<MrnQueue, Error>::Ok(MrnQueue {}), ]
      $($($rest)*)?
    )
  };
  (@as_tuple [ $($out:tt)* ] MrnCreateAsyncPrimitives::Future $(, $($rest:tt)* )? ) => {
    mrn_create_primitives!(
      @as_tuple [ $($out)* ::core::result::Result::<MrnFuture, Error>::Ok(MrnFuture {}), ]
      $($($rest)*)?
    )
  };
}

// end maroon `library` functions section

// fibers definition

fiber!("minimalRoot", {
  fn main() {
    println!("hello");
    match mrn_create_primitives!(vec![
      MrnCreateAsyncPrimitives::Queue { name: "rootQueue".to_string(), public: false },
      MrnCreateAsyncPrimitives::Future,
      MrnCreateAsyncPrimitives::Future,
    ]) {
      (Ok(mut queue), Ok(mut future_1), Ok(mut future_2)) => {
        println!("created queues");
      }
      (Err(err_1), Err(err_2), Err(err_3)) => {
        println!("{:?} {:?} {:?}", err_1, err_2, err_3);
      }
      _ => {}
    }

    println!("return");
  }
});

fiber!("minimalRoot2", {
  fn main() {
    println!("hello");
  }
});

/*
a bit future example. but now I'm focusing with smth easier

fiber("testRootFiber") {
    fn main {
        let (root_queue: Queue) = create_queues(vec![CreateInfo{name: "rootQueue", public: false}]);
        create_fibers(
            testCalculator(root_queue),
            testCalculator(root_queue),
        );
        let futures = create_futures(2);
        let f1: Future<u64> = futures[0];
        let f2: Future<u64> = futures[1];

        root_queue.send(TestCalculatorTask{
            a: 10,
            b: 20,
            responseFuture: f1,
        });
        root_queue.send(TestCalculatorTask{
            a: 20,
            b: 2,
            responseFuture: f2,
        });

        let res2: u64 = f2.await;
        let res1: u64 = f1.await;

        debug(res2, res1);
    }
}

fiber("testCalculator") {
    fn main(queue: Queue) {
        let (request) = select(queue);
        let res = request.a * request.b;
        request.responseFuture.resolve(res);
    }
}

struct TestCalculatorTask {
    a: u64,
    b: u64,
    responseFuture: Future,
}

/// Below - system provided types:

struct Queue {}

impl Queue {
    fn send(message) {}
}
struct Future {}

/// Below - system provided functions:
/// that will provide some runtime API
/// + will be used as a stop/pause point
/// all the code that we have in between will be wrapped into State::RustBlock

struct CreateInfo {
    name: String,
    public: bool,
}
fn create_queues(create_info: Vec<CreateInfo>) -> Vec<Queue> {}

fn create_futures(amount: usize) -> Vec<Future>{}

*/
