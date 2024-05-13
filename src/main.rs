use std::collections::HashMap;
use std::process::exit;
use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Serialize, Deserialize)]
struct FiaConfig {
    foo: String,
    bar: Baz,
}
impl FiaConfig {
    fn set<'a, 'b>(&'a mut self, mut path: impl Iterator<Item = &'b str>, arg: String) {
        match path.next() {
            Some("foo") => {
                self.foo = arg;
            }
            Some("bar") => {
                self.bar.set(path, arg);
            }
            Some(k) => error(format!("no such {k}")),
            None => {
                *self = serde_qs::from_str(&arg).unwrap_or_else(|e| error(format!("{e}")))
            }
        }
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
struct Baz {
    baz: String,
}
impl Baz {
    fn set<'a, 'b>(&'a mut self, mut path: impl Iterator<Item = &'b str>, arg: String) {
        match path.next() {
            Some("baz") => {
                self.baz = arg;
            }
            Some(k) => error(format!("no such {k}")),
            None => {
                *self = serde_qs::from_str(&arg).unwrap_or_else(|e| error(format!("config format error\n{e}")))
            }
        }
    }
}

enum FiaAction {
    Help,
    Reconfigure(Box<dyn FnOnce(&mut FiaConfig)>),
    HelloWorld(Option<String>),
}

fn update_in_place<T>(a: &mut T, f: impl FnOnce(T) -> T) {
    use std::ptr;
    unsafe {
        let a = a as *mut T;
        a.write(f(a.read()));
    }
}

impl FiaAction {
    fn flag(&mut self, c: char, args: &mut impl Iterator<Item = String>) {
        use FiaAction::*;
        match (self, c) {
            (HelloWorld(ref mut m), 'm') if m.is_none() => match args.next() {
                Some(n) => { m.insert(n); },
                None    => error("need argument for Wm")
            },
            (Reconfigure(f), 'c') => {
                let mut p: Vec<&str> = vec![];
                loop {
                    match args.next() {
                        None => error("need path element or = for Rc"),
                        Some(k) => {
                            if k == "=" {
                                match args.next() {
                                    None => error("need value for Rc"),
                                    Some(v) => update_in_place(f, |f| Box::new(|c| {
                                        f(c);
                                        c.set(p.iter().copied(), v);
                                        for e in p {
                                            // safety? absolutely not!
                                            unsafe {
                                                Box::<str>::from_raw(std::mem::transmute(e));
                                            }
                                        }
                                    }))
                                };
                                break
                            } else {
                                p.push(Box::leak(k.into()));
                            }
                        }
                    }
                }
            },
            (Reconfigure(f), 'p') => {
                update_in_place(f, |f| Box::new(|c| {
                    f(c);
                    println!("{c:?}");
                }))
            },
            (Reconfigure(f), 'd') => {
                update_in_place(f, |f| Box::new(|c| {
                    f(c);
                    println!("{}", serde_qs::to_string(c).unwrap_or_else(|e| error(format!("failed to dump config\n{e}"))));
                }))
            },
            _ => error(format!("unexpected option '{c}'"))
        };
    }
    fn run(self, state: &mut FiaState) {
        use FiaAction::*;
        match self {
            Help => {
                error("help is not and will likely never implemented. you're on your own.");
            }
            Reconfigure(f) => {
                f(&mut state.config);
            }
            HelloWorld(s) => {
                println!("{}", unwrap_borrow_or::<str>(&s, &"Hello, world!"));
            }
        };
    }
}

#[derive(Default)]
struct FiaState {
    config: FiaConfig,
    actions: Vec<FiaAction>
}

fn error(msg: impl std::borrow::Borrow<str>) -> ! {
    eprintln!("[1;31merror[0m: {}", msg.borrow().replace("\n", "\n     - "));
    exit(1)
}

fn unwrap_borrow_or<'a, T: ?Sized>(opt: &'a Option<impl std::borrow::Borrow<T> + 'a>, val: &'a (impl std::borrow::Borrow<T> + 'a)) -> &'a T {
    match opt {
        Some(val) => val.borrow(),
        None      => val.borrow(),
    }
}

fn main() -> ! {
    let mut state: FiaState = FiaState::default();
    let mut args = std::env::args();
    let progname: std::borrow::Cow<str> = args.next().map(Into::into).unwrap_or(env!("pname").into());
    for c in args.next().unwrap_or_default().chars() {
        match c {
            'a'..='z' => match state.actions.last_mut() {
                Some(a) => {
                    a.flag(c, &mut args)
                },
                None => error(format!("option flag '{c}' must come after action flag"))
            },
            '?' => state.actions.push(FiaAction::Help),
            'W' => state.actions.push(FiaAction::HelloWorld(None)),
            'R' => state.actions.push(FiaAction::Reconfigure(Box::new(|_| {}))),
            _   => error(format!("unknown flag '{c}'"))
        }
    }
    let actions: Vec<FiaAction> = state.actions.drain(..).collect();
    for a in actions {
        a.run(&mut state);
    }
    exit(0)
}