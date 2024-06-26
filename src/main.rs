#![allow(warnings)]

use std::collections::HashMap;
use std::process::exit;
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use resolve_path::PathResolveExt as _;
use std::fs::canonicalize;
use url::Url;
use std::io::{stdout, IsTerminal};

#[derive(Debug, Serialize, Deserialize)]
struct Repo {
    origin: Url,
}
trait Set: for<'de> Deserialize<'de> {
    fn set<'a, 'b>(&'a mut self, path: impl Iterator<Item = &'b str>, arg: String);
}
impl<T: Set> Set for Vec<T> {
    fn set<'a, 'b>(&'a mut self, mut path: impl Iterator<Item = &'b str>, arg: String) {
        match path.next() {
            None if arg == "" => {
                self.drain(..);
            },
            None => {
                error("overwriting vectors is unsupported");
            },
            Some(x) => match x.parse() {
                Ok(x) => if x == self.len() {
                    if let None = path.next() {
                        if arg != "" {
                            self.push(serde_qs::from_str(&arg).unwrap_or_else(|e| error(format!("{e}"))));
                        }
                    } else {
                        error("cannot append updates to a vec")
                    }
                } else if x > self.len() {
                    error("vec assignments must be at most one past the end") 
                } else {
                    let mut path = path.peekable();
                    if let None = path.peek() {
                        if arg == "" {
                            self.remove(x);
                        } else {
                            self[x] = serde_qs::from_str(&arg).unwrap_or_else(|e| error(format!("{e}")));
                        }
                    } else {
                        self[x].set(path, arg)
                    }
                },
                Err(e) => error(format!("invalid index {x}\n{e}")),
            }
        }
    }
}
impl Set for Repo {
    fn set<'a, 'b>(&'a mut self, mut path: impl Iterator<Item = &'b str>, arg: String) {
        match path.next() {
            Some("origin") => {
                self.origin = Url::parse(&arg).unwrap_or_else(|e| error(format!("invalid url for origin\n{e}")))
            },
            Some(k) => error(format!("no such {k}")),
            None => {
                *self = serde_qs::from_str(&arg).unwrap_or_else(|e| error(format!("{e}")))
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
#[repr(transparent)]
struct Jobs(usize);
impl Default for Jobs {
    fn default() -> Jobs {
        Jobs(num_cpus::get())
    }
}
impl Into<usize> for Jobs {
    fn into(self) -> usize {
        self.0
    }
}
impl From<usize> for Jobs {
    fn from(j: usize) -> Self {
        Jobs(j)
    }
}
impl std::str::FromStr for Jobs {
    type Err = <usize as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "+" {
            Ok(Default::default())
        } else {
            s.parse().map(Self)
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct FiaConfig {
    #[serde(default)]
    jobs: Jobs,
    #[serde(default)]
    repos: Vec<Repo>,
}
impl Set for FiaConfig {
    fn set<'a, 'b>(&'a mut self, mut path: impl Iterator<Item = &'b str>, arg: String) {
        match path.next() {
            // Some("figura_dir") => {
            //     self.figura_dir = canonicalize(arg).unwrap_or_else(|e| error(format!("cannot canonicalize figura_dir\n{e}")));
            // }
            Some("jobs") => {
                self.jobs = arg.parse().unwrap_or_else(|e| error(format!("invalid integer for jobs\n{e}")));
            }
            Some("repos") => {
                self.repos.set(path, arg);
            },
            Some(k) => error(format!("no such {k}")),
            None => {
                *self = serde_qs::from_str(&arg).unwrap_or_else(|e| error(format!("{e}")))
            }
        }
    }
}

enum FiaAction {
    Help,
    Reconfigure(Box<dyn FnOnce(&mut FiaConfig)>),
    HelloWorld(Option<String>),
    // CreateAvatar(&str)
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
            (Reconfigure(f), 'q') => {
                update_in_place(f, |f| Box::new(|c| {
                    f(c);
                    *c = Default::default();
                }))
            },
            (Reconfigure(f), 'l') => {
                update_in_place(f, |f| Box::new(|c| {
                    let Some(mut h) = std::env::home_dir() else { error("homeless :(") };
                    h.push(".config/fia");
                    println!("{}", h.display());
                }))
            },
            (Reconfigure(f), 'w') => {
                update_in_place(f, |f| Box::new(|c| {
                    let Some(mut h) = std::env::home_dir() else { error("homeless :(") };
                    h.push(".config");
                    std::fs::create_dir_all(&h).unwrap_or_else(|e| error(format!("failed to create config directory\n{e}")));
                    h.push("fia");
                    std::fs::write(h, serde_qs::to_string(c).unwrap_or_else(|e| error(format!("failed to dump config\n{e}")))).unwrap_or_else(|e| error(format!("failed to write config\n{e}")));
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
    eprintln!(
        "{} {}",
        if stdout().is_terminal() {
            "[1;31merror:[0m"
        } else {
            "error:"
        },
        msg.borrow().replace(
            "\n",
            if stdout().is_terminal() {
                "\n     [1;31m-[0m "
            } else {
                "\n     - "
            }
        )
    );
    exit(1)
}
fn warn(msg: impl std::borrow::Borrow<str>) -> () {
    eprintln!(
        "{} {}",
        if stdout().is_terminal() {
            "[1;33mwarn:[0m"
        } else {
            "warn:"
        },
        msg.borrow().replace(
            "\n",
            if stdout().is_terminal() {
                "\n    [1;33m-[0m "
            } else {
                "\n     - "
            }
        )
    );
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
    if let Some(mut h) = std::env::home_dir() {
        h.push(".config/fia");
        match std::fs::read_to_string(h) {
            Ok(t) => match serde_qs::from_str(&t) {
                Ok(d) => {
                    state.config = d;
                }
                Err(e) => {
                    warn(format!("persistent config is invalid\n{e}"))
                }
            },
            Err(e) => warn(format!("cannot open config\n{e}\nto regenerate config, run `{progname} Rw`"))
        }
    }
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
