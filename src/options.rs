use std::env;

pub(crate) struct Options {
    // TODO : instead of 2 bool for link and assembling, having an enum to have the target of the tool : linking, assembling, etc
    pub should_link : bool,
    pub should_assemble : bool,
    pub source_file : String,
    pub output_file : String,
}

// TODO : better error messages
pub(crate) fn parse_options() -> Options {
    let mut should_link = true;
    let mut should_assemble = true;
    let mut f = None;
    let mut output_file = "a.out".to_string();
    let mut args_iter = env::args().skip(1);
    while let Some(arg) = args_iter.next() {
        if arg == "-S" {
            should_assemble = false;
            should_link = false;
        } else if arg == "-c" {
            should_link = false;
        } else if arg == "-o" {
            output_file = args_iter.next().expect("missing output file after -o");
        } else {
            f = Some(arg);
        }
    }
    let f = f.expect("missing arg");
    Options {
        should_link,
        should_assemble,
        source_file: f,
        output_file,
    }
}