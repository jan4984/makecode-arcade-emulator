//mod emulator;
use mpsc::Receiver;
use std::collections::HashMap;


use std::sync::mpsc::{self, SyncSender};

use std::{thread, fmt};

use crate::emulator;
use crate::emulator::game::{BMP_WIDTH, BMP_HEIGHT, KeyCode};
use crate::v8_binding::Runtime;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;



#[derive(Debug)]
pub enum Event {
    KeyUp(KeyCode),
    KeyDown(KeyCode),
    KeyRepeat(KeyCode),
    Load(Project),
    Unload,
    Exit,
}

#[derive(Debug)]
pub struct Project{
    pub sources:HashMap<String, String>,
}

unsafe impl Send for Project{}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

pub struct Engine {
    pub fb_rx: Receiver<[u32; (BMP_WIDTH * BMP_HEIGHT) as usize]>,
    pub event_tx: SyncSender<Event>,
    pub tick_tx: SyncSender<u64>,
}

unsafe impl Sync for Engine {}

impl Engine {
    pub fn new(_fps: u32, _audio_hz: u32) -> Self {
        let (fb_tx, fb_rx) =
            std::sync::mpsc::sync_channel::<[u32; (BMP_WIDTH * BMP_HEIGHT) as usize]>(2);
        let (tick_tx, tick_rx) = std::sync::mpsc::sync_channel::<u64>(1);
        let (event_tx, event_rx) = std::sync::mpsc::sync_channel::<Event>(5);
        thread::spawn(move || {
            let mut runtime = Runtime::new();
            let mut prj:Option<Project> = None;
            
            'main: loop {                
                match tick_rx.recv() {
                    Err(e) => {
                        println!("game engine receive event:{}", e);
                        break 'main;
                    }
                    Ok(micro_sec) => {
                        runtime.update(micro_sec)
                    },
                }

                'events: loop {
                    match event_rx.try_recv() {
                        Err(mpsc::TryRecvError::Empty) => break 'events,
                        Err(e) => {
                            println!("game engine receive event:{}", e);
                            break 'main;
                        },
                        Ok(evt) => match evt {
                            //TODO: process inputs
                            Event::Exit => {break 'main;},
                            Event::Unload=>{
                                runtime.reset();
                                if let Some(p) = prj.as_ref() {
                                    runtime.run_script(p.sources.get("main.ts").unwrap());
                                }
                            },
                            Event::Load(v)=>{               
                                prj = Some(v);                 
                                runtime.run_script(prj.as_ref().unwrap().sources.get("main.ts").unwrap());
                            },
                            Event::KeyDown(_)|Event::KeyRepeat(_)|Event::KeyUp(_)=>{
                                runtime.process_events(evt);
                            },
                        },
                    }
                }

                runtime.process_overlap_check();

                let mut canvas = emulator::resource::Canvas::new();
                runtime.draw(&mut canvas);

                match fb_tx.send(canvas.0) {
                    Err(err) => {
                        println!("receiver disconnected:{}", err);
                        break 'main;
                    }
                    _ => {}
                };                
            }
        });
        Engine {
            fb_rx,
            event_tx,
            tick_tx,
        }
    }
}
