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

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((mamma_sexp, mut mamma_iter)) = self.stack.pop_front() {
            // if there is juice left in the iterator
            if let Some(next_item) = mamma_iter.next() {
                // if we have successfully found a needle
                if let Some(next_sexp) = mamma_sexp.replace_first(&self.needle, &next_item) {
                    // there might be more juice in the mamma_iter,
                    // so push it back on the stack so that we try
                    // to process it again
                    self.stack.push_front((mamma_sexp, mamma_iter));

                    // we want to split off the next one
                    let new_iter = (self.spawn_iterator)();
                    self.stack.push_front((next_sexp, new_iter));

                    self.next()
                } else {
                    // otherwise (no needle), then we have a complete sexp
                    // we can simply return it
                    Some(mamma_sexp)
                }
            } else {
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
        // let (res, success) = self.replace_first_help(needle, new);
        // success.then(|| res)
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
