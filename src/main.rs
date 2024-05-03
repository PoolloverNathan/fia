use std::collections::HashMap;
use std::process::exit;

enum FiaAction {
    Help,
    HelloWorld(Option<String>),
}

impl FiaAction {
    fn flag(&mut self, c: char, args: &mut impl Iterator<Item = String>) {
        match (self, c) {
            (FiaAction::HelloWorld(ref mut m), 'm') if m.is_none() => match args.next() {
                Some(n) => m.insert(n),
                None    => error("option Wm expects an argument")
            },
            _ => error(format!("unexpected option '{c}'"))
        };
    }
}

#[derive(Default)]
struct FiaState {
    actions: Vec<FiaAction>
}

fn error(msg: impl std::borrow::Borrow<str>) -> ! {
    eprintln!("[1;31merror[0m: {}", msg.borrow().replace("\n", "\n     | "));
    exit(1)
}

fn unwrap_borrow_or<'a, T: ?Sized>(opt: &'a Option<impl std::borrow::Borrow<T> + 'a>, val: &'a (impl std::borrow::Borrow<T> + 'a)) -> &'a T {
    match opt {
        Some(val) => val.borrow(),
        None      => val.borrow(),
    }
}

#[tokio::main]
async fn main() -> ! {
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
            'H' => state.actions.push(FiaAction::Help),
            'W' => state.actions.push(FiaAction::HelloWorld(None)),
            _   => error(format!("unknown flag '{c}'"))
        }
    }
    for a in state.actions {
        match a {
            FiaAction::Help => {
                error("unfortunately you are helpless")
            }
            FiaAction::HelloWorld(a) => {
                println!("{}", unwrap_borrow_or::<str>(&a, &"Hello, world!"));
            }
        }
    }
    exit(0)
}