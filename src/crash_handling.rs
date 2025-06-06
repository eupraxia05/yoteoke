use minidumper_child::{MinidumperChild, ClientHandle};
use std::{io::Write, path::Path};

pub fn run_handler() -> CrashHandlerGuard {
  let guard = MinidumperChild::new()
  .on_minidump(|buffer: Vec<u8>, _: &Path| {
    if let Err(e) = std::fs::File::create("crash.dmp").unwrap().write_all(&buffer){
      println!("error writing crash dump: {:?}", e);
    } else {
      println!("crashed! dump written to crash.dmp")
    }
  })
  .spawn();

  CrashHandlerGuard { _handle: guard.unwrap() }
}

pub struct CrashHandlerGuard {
  _handle: ClientHandle
}