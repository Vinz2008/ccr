// TODO : write my own preprocessor ?

use std::{io::{BufWriter, Write}, process::{Command, Stdio}};

pub(crate) fn preprocess(content : String) -> String {
    let mut cmd = Command::new("cpp");
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());
    let mut cmd_child = cmd.spawn().unwrap();

    {
        let mut writer = BufWriter::new(cmd_child.stdin.as_mut().unwrap());
        writer.write_all(content.as_bytes()).unwrap();
    }

    let output = cmd_child.wait_with_output().unwrap();
    let preprocessed_output = str::from_utf8(&output.stdout).unwrap();
    println!("preprocessed_output : {}", preprocessed_output);
    preprocessed_output.to_string()
}