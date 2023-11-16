use itertools::Itertools;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Sexp {
    Atom(String),
    List(Vec<Self>),
}

impl Sexp {
    fn subst(self, hole: String, val: Sexp) -> Sexp {
        match self {
            Sexp::Atom(s) if s == hole => val.clone(),
            Sexp::Atom(_) => self,
            Sexp::List(vals) => Sexp::List(
                vals.into_iter()
                    .map(|s| s.subst(hole.clone(), val.clone()))
                    .collect(),
            ),
        }
    }

    fn subst_one(self, hole: String, val: Sexp) -> (Sexp, bool) {
        match self {
            Sexp::Atom(s) if s == hole => (val.clone(), true),
            Sexp::Atom(_) => (self, false),
            Sexp::List(vals) => {
                let (lst, done) = vals
                    .into_iter()
                    .fold((vec![], false), |(mut acc, done), el| {
                        if done {
                            acc.push(el);
                            (acc, done)
                        } else {
                            // if the sexp is successful, then done becomes true
                            // which will mean nothing else is changed
                            let (new_sexp, done) = el.subst_one(hole.clone(), val.clone());
                            acc.push(new_sexp);
                            (acc, done)
                        }
                    });
                (Sexp::List(lst), done)
            }
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

    fn iter(self) -> Box<dyn Iterator<Item = Sexp>> {
        match self {
            Workload::Set(v) => Box::new(v.into_iter()),
            Workload::Plug(wkld, hole, pegs) => Box::new(
                // for each workload
                wkld.iter()
                    // make iterator local copies of the things that we need
                    .map(move |w| (w, hole.clone(), pegs.clone()))
                    .map(|(w, hole, pegs)| {
                        // for each peg
                        pegs.iter()
                            .map(move |peg| w.clone().subst_one(hole.clone(), peg).0)
                    })
                    .flatten(),
            ),
            // Workload::Plug(wkld, hole, pegs) => Box::new(
            //     pegs.iter()
            //         .map(move |peg| (wkld.clone(), hole.clone(), peg))
            //         .map(|(wkld, hole, peg)| {
            //             wkld.iter()
            //                 .map(move |expr| expr.subst(hole.clone(), peg.clone()))
            //         })
            //         .flatten(),
            // ),
        }
    }

    // .flat_map(|p| {
    //     wkld.clone()
    //         .iter()
    //         .map(|s| s.clone().subst(hole.clone(), p.clone()))
    // })
}

fn main() {
    let v = Workload::Set(vec![
        Sexp::Atom("0".to_string()),
        Sexp::Atom("1".to_string()),
    ]);
    let expr = Sexp::List(vec![
        Sexp::Atom("+".to_string()),
        Sexp::Atom("A".to_string()),
        Sexp::Atom("A".to_string()),
    ]);
    let wkld = Workload::Set(vec![expr]).plug("A", v);
    // println!("{:?}", wkld.iter().nth(2));
    for s in wkld.iter() {
        println!("{s:?}");
    }

    let zo = vec![Sexp::Atom("0".to_string()), Sexp::Atom("1".to_string())];

    for x in [zo.iter(), zo.iter()].iter().multi_cartesian_product() {
        println!("{x:?}");
    }
}
