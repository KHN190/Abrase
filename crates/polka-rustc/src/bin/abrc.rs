use std::process::ExitCode;

const USAGE: &str = "\
usage: abrc <file.pk> [-o <out.rs>]
    Compiles a Polka cartridge to standalone Rust on stdout (or -o <file>).
    Link the emitted program against the `myriad` rlib to run it.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mut input: Option<String> = None;
    let mut output: Option<String> = None;
    let mut it = args.iter().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-o" => match it.next() {
                Some(o) => output = Some(o.clone()),
                None => { eprint!("{}", USAGE); return ExitCode::from(64); }
            },
            "-h" | "--help" => { print!("{}", USAGE); return ExitCode::SUCCESS; }
            _ => input = Some(a.clone()),
        }
    }
    let Some(path) = input else { eprint!("{}", USAGE); return ExitCode::from(64); };

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => { eprintln!("abrc: cannot read {}: {}", path, e); return ExitCode::from(66); }
    };
    let module = match polka::cartridge::read_pk(&bytes) {
        Ok(m) => m,
        Err(e) => { eprintln!("abrc: not a valid .pk cartridge: {:?}", e); return ExitCode::from(65); }
    };
    let rust = match polka_rustc::transpile_module(&module) {
        Ok(s) => s,
        Err(e) => { eprintln!("abrc: {}", e); return ExitCode::from(1); }
    };
    match output {
        Some(o) => match std::fs::write(&o, rust) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => { eprintln!("abrc: cannot write {}: {}", o, e); ExitCode::from(73) }
        },
        None => { print!("{}", rust); ExitCode::SUCCESS }
    }
}
