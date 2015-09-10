//! Hierarchical organization of timely dataflow graphs.
//!

use std::rc::Rc;
use std::cell::RefCell;

use progress::{Timestamp, Operate, Subgraph};
use progress::nested::{Source, Target};
use timely_communication::Allocate;

pub mod root;
pub mod child;

pub use self::child::Child;
pub use self::root::Root;

/// The fundamental operations required to add and connect operators in a timely dataflow graph.
///
/// Importantly, this is often a *shared* object, backed by a `Rc<RefCell<>>` wrapper. Each method
/// takes a shared reference, but can be thought of as first calling .clone() and then calling the
/// method. Each method does not hold the `RefCell`'s borrow, and should prevent accidental panics.
pub trait Scope : Allocate+Clone {

    /// The timestamp associated with data in this scope.
    type Timestamp : Timestamp;

    /// A useful name describing the scope.
    fn name(&self) -> String;
    fn addr(&self) -> Vec<usize>;

    /// Connects a source of data with a target of the data. This only links the two for
    /// the purposes of tracking progress, rather than effect any data movement itself.
    fn add_edge(&self, source: Source, target: Target);

    /// Adds a child `Operate` to the builder's scope. Returns the new child's index.
    fn add_operator<SC: Operate<Self::Timestamp>+'static>(&self, scope: SC) -> usize;

    /// Creates a new `Subgraph` with timestamp `T`. Used by `scoped`, but unlikely to be
    /// commonly useful to end users.
    fn new_subscope<T: Timestamp>(&mut self) -> Subgraph<Self::Timestamp, T>;

    fn new_identifier(&mut self) -> usize;

    /// Creates a `Subgraph` from a closure acting on a `Child` scope, and returning
    /// whatever the closure returns.
    ///
    /// Commonly used to create new timely dataflow subgraphs, either creating new input streams
    /// and the input handle, or ingressing data streams and returning the egresses stream.
    ///
    /// # Examples
    /// ```ignore
    /// use timely::dataflow::*;
    /// use timely::dataflow::operators::*;
    ///
    /// timely::execute(std::env::args(), |root| {
    ///     // must specify types as nothing else drives inference.
    ///     let input = root.scoped::<u64,_,_>(|child1| {
    ///         let (input, stream) = child1.new_input::<String>();
    ///         let output = child1.scoped::<u32,_,_>(|child2| {
    ///             child2.enter(&stream).leave()
    ///         });
    ///         input
    ///     });
    /// });
    /// ```
    fn scoped<T: Timestamp, R, F:FnOnce(&mut Child<Self, T>)->R>(&mut self, func: F) -> R {
        let subscope = Rc::new(RefCell::new(self.new_subscope()));
        let mut builder = Child {
            subgraph: subscope,
            parent: self.clone(),
        };
        
        let result = func(&mut builder);
        self.add_operator(builder.subgraph);
        result
    }
}