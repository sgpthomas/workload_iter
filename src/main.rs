use std::{collections::VecDeque, fmt::Display};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Sexp {
    Atom(String),
    List(Vec<Self>),
}

impl Display for Sexp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sexp::Atom(s) => write!(f, "{s}"),
            Sexp::List(list) => {
                write!(f, "(")?;
                for (i, el) in list.iter().enumerate() {
                    write!(f, "{el}")?;
                    if i < list.len() - 1 {
                        write!(f, " ")?;
                    }
                }
                write!(f, ")")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct SexpSubstIter<I, F>
where
    I: Iterator<Item = Sexp>,
    F: Fn() -> I,
{
    needle: String,
    spawn_iterator: F,
    stack: VecDeque<(Sexp, I)>,
}

impl<I, F> SexpSubstIter<I, F>
where
    I: Iterator<Item = Sexp>,
    F: Fn() -> I,
{
    fn new<S: ToString>(inital_sexp: Sexp, needle: S, spawn_iterator: F) -> Self {
        let initial_iter = spawn_iterator();
        SexpSubstIter {
            needle: needle.to_string(),
            spawn_iterator,
            stack: VecDeque::from([(inital_sexp, initial_iter)]),
        }
    }
}

impl<I, F> Iterator for SexpSubstIter<I, F>
where
    I: Iterator<Item = Sexp>,
    F: Fn() -> I,
{
    type Item = Sexp;

    /// Intuitively, the thing that we want to do is perform a traversal of the leaves
    /// of the following tree. Each level of the tree represents substituting the hole
    /// variable (`A` in this case), with every instance of some other iterator.
    ///
    /// ```
    ///             (+ A A)
    ///            /       \
    ///     (+ 0 A)         (+ 1 A)
    ///       / \             / \
    /// (+ 0 0) (+ 0 1) (+ 1 0) (+ 1 1)
    /// ```
    ///
    /// The thing that makes it tricky to write this traversal in a lazy way is that we
    /// don't know what this tree will look like up-front; it's lazily produced by
    /// another iterator filling in the values of `A`.
    ///
    /// The trick is that we can use a stack to represent where we are in this tree
    /// traversal, making sure that we have enough information to unfold the next layer
    /// of the tree. Specifically, we can store an iterator for each layer of the tree,
    /// and a template to generate the next layer deep in the tree.
    ///
    /// Pictorially, it will look something like this:
    ///
    /// ```
    /// 1.
    /// (+ A A)
    ///
    /// 2.
    /// (+ A A)
    ///    |
    /// (+ 0 A)
    ///
    /// 3.
    /// (+ A A)
    ///    |
    /// (+ 0 A)
    ///    |
    /// (+ 0 0)
    ///
    /// 4.
    ///     (+ A A)
    ///        |
    ///     (+ 0 A)
    ///      /  \
    /// (+ 0 0) (+ 0 1)
    /// ```
    ///
    /// We implement this by maintaining work on a stack. An item of work is a template
    /// expression along with an iterator. Each item of work represents the set of
    /// expressions obtained by replacing the first instance of the needle variable with
    /// every item of the iterator. For example, `(+ 0 A), [0, 1, 2]` represents the set
    /// of expressions `(+ 0 0) (+ 0 1) (+ 0 2)`. By moving through the iterator, you
    /// can lazily generate nodes of the tree in the layer below the template
    /// expression.
    ///
    /// In this way, we can use these items of work to keep track of where we have
    /// explored in our tree search so far: each item of work keeps track of where we
    /// are at each layer of the tree. Once there are no more instances of the needle
    /// variable, we can yield the item from the iterator.
    ///
    /// Here is the actual algorithm:
    /// Given an item of work, we
    ///
    /// 1. Try to progress the search horiztonally in the tree. We do this by checking
    ///    the iterator associated with an item of work. If there are no items left, we
    ///    have reached the end of this layer. We simply move onto the next item of
    ///    work. If there are items left in the iterator,
    /// 2. We try to spawn a deeper layer of the search. We do this by starting a new
    ///    iterator for the child of the previous layer. If we can't go any deeper, then
    ///    we know that we are at a leaf, and can yield this item. If we can, we make
    ///    sure to put this item at the front of the stack so that it is processed on
    ///    the next iterator. This results in a depth-first traversal of the tree.
    ///
    /// Let's walk through how this works for this workload: `(+ A A).plug(A, {0, 1, 2})`
    ///
    /// The stack starts off with
    ///
    /// ```
    /// (+ A A), [0, 1, 2]
    /// ```
    ///
    /// ```
    /// (+ 0 A) [0, 1, 2]
    /// (+ A A) [1, 2]
    /// ```
    ///
    /// ```
    /// (+ 0 0) [0, 1, 2]
    /// (+ 0 A) [1, 2]
    /// (+ A A) [1, 2]
    /// ```
    ///
    /// Produced!: `(+ 0 0)`
    ///
    /// ```
    /// (+ 0 A) [1, 2]
    /// (+ A A) [1, 2]
    /// ```
    ///
    /// ```
    /// (+ 0 1) [0, 1, 2]
    /// (+ 0 A) [2]
    /// (+ A A) [1, 2]
    /// ```
    ///
    /// Produced!: `(+ 0 1)`
    ///
    /// ```
    /// (+ 0 2) [0, 1, 2]
    /// (+ 0 A) []
    /// (+ A A) [1, 2]
    /// ```
    ///
    /// Produced!: `(+ 0 2)`
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((parent_sexp, mut parent_iter)) = self.stack.pop_front() {
            // if there is juice left in the iterator
            if let Some(next_item) = parent_iter.next() {
                // try to go deeper one layer by replacing the first instance of the
                // needle with the item we got from the iterator
                if let Some(child_sexp) = parent_sexp.replace_first(&self.needle, &next_item) {
                    // there might be more juice in the parent_iter,
                    // so push it back on the stack so that we try
                    // to process it again
                    self.stack.push_front((parent_sexp, parent_iter));

                    // next we want to spawn a new iterator representing one layer
                    // deeper in the search. we want to make sure that this item
                    // is the next item processed on the stack so that we perform
                    // a depth-first traversal of the tree.
                    let child_iter = (self.spawn_iterator)();
                    self.stack.push_front((child_sexp, child_iter));

                    self.next()
                } else {
                    // otherwise (no needle), we are at a leaf and all instances
                    // of the needle are fully instantiated. we can yield this
                    // item from the iterator
                    Some(parent_sexp)
                }
            } else {
                // we are done with this layer of the tree. continue processing
                // the next item on the stack
                self.next()
            }
        } else {
            None
        }
    }
}

impl Sexp {
    fn first(&mut self, needle: &str) -> Option<&mut Self> {
        match self {
            Sexp::Atom(a) if a == needle => Some(self),
            Sexp::Atom(_) => None,
            Sexp::List(list) => list.into_iter().find_map(|s| s.first(needle)),
        }
    }

    fn replace_first(&self, needle: &str, new: &Sexp) -> Option<Self> {
        let mut copy = self.clone();
        if let Some(ptr) = copy.first(needle) {
            *ptr = new.clone();
            Some(copy)
        } else {
            None
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Workload {
    Set(Vec<Sexp>),
    Plug(Box<Self>, String, Box<Self>),
}

impl Workload {
    fn plug(self, hole: &str, pegs: Self) -> Workload {
        Workload::Plug(Box::new(self), hole.to_string(), Box::new(pegs))
    }
}

impl IntoIterator for Workload {
    type Item = Sexp;
    type IntoIter = Box<dyn Iterator<Item = Sexp>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Workload::Set(v) => Box::new(v.into_iter()),
            Workload::Plug(wkld, hole, pegs) => Box::new(
                wkld.into_iter()
                    .map(move |sexp| (sexp, hole.clone(), pegs.clone()))
                    .map(|(sexp, hole, pegs)| {
                        SexpSubstIter::new(sexp, hole, move || pegs.clone().into_iter())
                    })
                    .flatten(),
            ),
        }
    }
}

fn main() {
    let v = Workload::Set(vec![
        Sexp::Atom("0".to_string()),
        Sexp::Atom("1".to_string()),
        Sexp::Atom("2".to_string()),
    ]);
    let v0 = Workload::Set(vec![
        Sexp::Atom("a".to_string()),
        Sexp::Atom("b".to_string()),
    ]);
    let expr = Sexp::List(vec![
        Sexp::Atom("+".to_string()),
        Sexp::Atom("A".to_string()),
        Sexp::Atom("B".to_string()),
    ]);
    let wkld = Workload::Set(vec![expr]).plug("A", v).plug("B", v0);

    for v in wkld {
        println!("recv: {v}");
    }
}
