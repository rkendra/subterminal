use subterminal::{self, Pty};
use std::io::prelude::*;
use std::fs::File;

#[test]
fn test_basic() -> std::io::Result<()> {
    let mut shell = Pty::spawn_shell()?;
    println!("Shell instance spawned");
    let expected = "[?2004h]0;ryanj@RYAN-ROG: ~/subterminal[01;32mryanj@RYAN-ROG[00m:[01;34m~/subterminal[00m$ echo hello
[?2004l
hello
[?2004h]0;ryanj@RYAN-ROG: ~/subterminal[01;32mryanj@RYAN-ROG[00m:[01;34m~/subterminal[00m$ [?2004l

exit
";
    let mut output: [u8; 4096] = [0; 4096];
    let num_read = shell.read(&mut output)?;
    let input = "echo hello\n".as_bytes();
    shell.write(input)?;
    println!("Sent echo command to shell");
    
    shell.read(&mut output[num_read..])?;
    let mut out = File::create("test.out")?;
    out.write(&output)?;
    assert_eq!(expected.as_bytes(), &output[..expected.as_bytes().len()]);

    Ok(())
}