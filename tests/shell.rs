use subterminal::{self, Pty, PtyIn, PtyOut};
use std::io::prelude::*;
use std::thread;
use std::sync::atomic;
use std::sync::mpsc;

fn reader(ostream: &mut PtyOut, stop: &atomic::AtomicBool, tx: mpsc::Sender<Vec<u8>>) -> std::io::Result<()> {
    while !(stop.load(atomic::Ordering::Relaxed)) {
        let mut buf: [u8; 4096] = [0; 4096];
        let bytes_read = ostream.read(&mut buf)?;
        let vec = buf[..bytes_read].to_vec();
        tx.send(vec).unwrap();
    }
    Ok(())
}
        
    
#[test]
fn test_echo() -> std::io::Result<()> {
    let expected = String::from("hello").into_bytes();
    let shell = Pty::spawn(String::from("echo hello"))?;
    let (tx, rx) = mpsc::channel();
    let mut term_output = shell.output;
    let stop = atomic::AtomicBool::new(false);
    thread::scope (|s| {
        let reader = s.spawn(|| reader(&mut term_output, &stop, tx));
        let output = rx.recv().unwrap();
        stop.store(true, atomic::Ordering::Relaxed);
        let _ = reader.join().unwrap();
        assert_eq!(expected, output);
    });
    Ok(())
}
