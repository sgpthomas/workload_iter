use std::collections::VecDeque;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Sexp {
    Atom(String),
    List(Vec<Self>),
}

#[derive(Clone)]
pub struct SexpSubstIter<I, F>
where
    I: Iterator<Item = Sexp>,
    F: Fn() -> I,
{
    needle: String,
    iterator_progenitor: F,
    stack: VecDeque<(Sexp, I)>,
}

impl<I, F> SexpSubstIter<I, F>
where
    I: Iterator<Item = Sexp>,
    F: Fn() -> I,
{
    fn new<S: ToString>(inital_sexp: Sexp, needle: S, iterator_progenitor: F) -> Self {
        let initial_iter = iterator_progenitor();
        SexpSubstIter {
            needle: needle.to_string(),
            iterator_progenitor,
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
                if let Some(next_sexp) = mamma_sexp.map_first(&self.needle, &next_item) {
                    // we want to split off the next one
                    let new_iter = (self.iterator_progenitor)();
                    self.stack.push_back((next_sexp, new_iter));

                    self.stack.push_back((mamma_sexp, mamma_iter));

                    self.next()
                } else {
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
    #[allow(unused)]
    fn find_all(&mut self, needle: &str) -> Vec<&mut Self> {
        match self {
            Sexp::Atom(s) if needle == s => vec![self],
            Sexp::Atom(_) => vec![],
            Sexp::List(list) => list
                .into_iter()
                .map(|v| v.find_all(needle))
                .flatten()
                .collect(),
        }
    }

    fn map_first_help(&self, needle: &str, new: &Sexp) -> (Self, bool) {
        match self {
            Sexp::Atom(s) if needle == s => (new.clone(), true),
            Sexp::Atom(_) => (self.clone(), false),
            Sexp::List(list) => {
                let (inner, success) =
                    list.into_iter()
                        .fold((vec![], false), |(mut acc, success), el| {
                            if !success {
                                let (res, success) = el.map_first_help(needle, new);
                                acc.push(res);
                                (acc, success)
                            } else {
                                acc.push(el.clone());
                                (acc, success)
                            }
                        });
                (Sexp::List(inner), success)
            }
        }
    }

    fn map_first(&self, needle: &str, new: &Sexp) -> Option<Self> {
        let (res, success) = self.map_first_help(needle, new);
        success.then(|| res)
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
        println!("{v:?}");
    }
}
