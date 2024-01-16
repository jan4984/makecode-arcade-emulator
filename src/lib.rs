#![allow(dead_code)]

mod emulator;
mod engine;
mod libretro;
mod v8_binding;

use std::{
    fs::File,
    io::Read,
    collections::HashMap,
    os::{raw::{c_char, c_uint, c_void}},    
    ptr
};

const FPS: u32 = 50;
const AUDIO_HZ: u32 = 16000;

use emulator::game::KeyCode;


use crate::libretro::bindings;

//because libretro not use an opaque context, we need global variables

static mut LOG_CB: bindings::retro_log_printf_t = None;
static mut VIDEO_CB: bindings::retro_video_refresh_t = None;
static mut INPUT_POLL_CB: bindings::retro_input_poll_t = None;
static mut INPUT_STATE_CB: bindings::retro_input_state_t = None;
static mut INPUT_STATES: Option<HashMap<u32, bool>> = None;
static mut ENVIRON_CB: bindings::retro_environment_t = None;
static mut AUDIO_CB: bindings::retro_audio_sample_t = None;

//retroarch must not keep the log string
static mut TMP_SPACE_FOR_LOG: Option<String> = None;

macro_rules! static_cptr {
    ($s:expr) => {
        std::concat!($s, "\0").as_ptr() as *const i8
    };
}

fn D(msg: &'static str) {
    unsafe {
        if LOG_CB.is_none() {
            println!("{}", msg);
            return;
        }
        LOG_CB.unwrap()(
            bindings::retro_log_level_RETRO_LOG_DEBUG,
            static_cptr!("%.*s\n"),
            msg.len(),
            msg.as_ptr() as *const i8,
        );
    }
}

fn I(msg: &'static str) {
    unsafe {
        if LOG_CB.is_none() {
            println!("{}", msg);
            return;
        }
        LOG_CB.unwrap()(
            bindings::retro_log_level_RETRO_LOG_INFO,
            static_cptr!("%.*s\n"),
            msg.len(),
            msg.as_ptr() as *const i8,
        );
    }
}

#[no_mangle]
extern "C" fn retro_api_version() -> u32 {
    bindings::RETRO_API_VERSION
}

fn get_input_desc(
    port: c_uint,
    device: c_uint,
    index: c_uint,
    id: c_uint,
    desc: &'static str,
) -> bindings::retro_input_descriptor {
    bindings::retro_input_descriptor {
        port,
        device,
        index,
        id,
        description: desc.as_ptr() as *const i8,
    }
}

//skip first 10 frames, for game warmup, because our games may not havs 'presss X to start' but directly go
static mut WARMUP_COUNDDOWN: i32 = 5;

fn tmp_c_str(s: String) -> &'static str {
    unsafe {
        TMP_SPACE_FOR_LOG = Some(s);
        return TMP_SPACE_FOR_LOG.as_ref().unwrap();
    }
}

extern "C" fn retro_set_next_tick_time(mut micro_sec: i64) {
    unsafe {
        if TEN_CC > 0 {
            I(tmp_c_str(format!("tick duration:{}", micro_sec)));
            TEN_CC -= 1;
        }

        if micro_sec > 0 {
            if WARMUP_COUNDDOWN > 0 {
                WARMUP_COUNDDOWN -= 1;
                micro_sec = 1000;
            }
            match ENGINE.as_ref().unwrap().tick_tx.send(micro_sec as u64) {
                Err(e) => I(tmp_c_str(format!("send tick to engine:{}", e))),
                Ok(_) => {}
            };
        }
    }
}

#[no_mangle]
extern "C" fn retro_unload_game() {
    D("retro_unload_game()");
    unsafe {
        match ENGINE
            .as_ref()
            .unwrap()
            .event_tx
            .send(engine::Event::Unload)
        {
            Err(e) => I(tmp_c_str(format!("send unload to engine:{}", e))),
            _ => {}
        };
    }
}

#[no_mangle]
extern "C" fn retro_load_game(info: *const bindings::retro_game_info) -> bool {
    D("retro_load_game()");
    let mut pixel_fmt = bindings::retro_pixel_format_RETRO_PIXEL_FORMAT_XRGB8888;
    unsafe {
        ENVIRON_CB.unwrap()(
            bindings::RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
            ptr::addr_of_mut!(pixel_fmt) as *mut c_void,
        );
    }
    let mut desc: [bindings::retro_input_descriptor; 7] = [
        get_input_desc(
            0,
            bindings::RETRO_DEVICE_JOYPAD,
            0,
            bindings::RETRO_DEVICE_ID_JOYPAD_LEFT,
            "Left",
        ),
        get_input_desc(
            0,
            bindings::RETRO_DEVICE_JOYPAD,
            0,
            bindings::RETRO_DEVICE_ID_JOYPAD_UP,
            "Up",
        ),
        get_input_desc(
            0,
            bindings::RETRO_DEVICE_JOYPAD,
            0,
            bindings::RETRO_DEVICE_ID_JOYPAD_DOWN,
            "Down",
        ),
        get_input_desc(
            0,
            bindings::RETRO_DEVICE_JOYPAD,
            0,
            bindings::RETRO_DEVICE_ID_JOYPAD_RIGHT,
            "Right",
        ),
        get_input_desc(
            0,
            bindings::RETRO_DEVICE_JOYPAD,
            0,
            bindings::RETRO_DEVICE_ID_JOYPAD_A,
            "A",
        ),
        get_input_desc(
            0,
            bindings::RETRO_DEVICE_JOYPAD,
            0,
            bindings::RETRO_DEVICE_ID_JOYPAD_B,
            "B",
        ),
        bindings::retro_input_descriptor {
            port: 0,
            device: bindings::RETRO_DEVICE_NONE,
            index: 0,
            id: 0,
            description: ptr::null::<i8>(),
        },
    ];

    let mut tick_cb = bindings::retro_frame_time_callback {
        reference: 1000000 as i64 / FPS as i64,
        callback: Some(retro_set_next_tick_time),
    };

    unsafe {
        ENVIRON_CB.unwrap()(
            bindings::RETRO_ENVIRONMENT_SET_FRAME_TIME_CALLBACK,
            ptr::addr_of_mut!(tick_cb) as *mut c_void,
        );
        //we need tick to drive engine go, or fb not filled
        retro_set_next_tick_time(1000);

        ENVIRON_CB.unwrap()(
            bindings::RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS,
            ptr::addr_of_mut!(desc) as *mut c_void,
        );

        match info.as_ref() {
            Some(p) => {
                if !p.data.is_null() {
                    let mut sl = std::slice::from_raw_parts(p.data as *const u8, p.size as usize);
                    if sl[sl.len() - 1] == 0 {
                        sl = &sl[..sl.len() - 1];
                    }
                    D(tmp_c_str(format!("to load game data size: {}", sl.len())));
                    //let src = String::from(std::str::from_utf8(sl).unwrap());
                    //D(tmp_c_str(format!("to load game data: {}", src)));
                    let prj = match loadPNG(sl) {
                        Ok(p) => p,
                        Err(msg) => {
                            I(tmp_c_str(msg));
                            return false;
                        }
                    };
                    match ENGINE
                        .as_ref()
                        .unwrap()
                        .event_tx
                        .send(engine::Event::Load(prj))
                    {
                        Err(e) => I(tmp_c_str(format!("send load to engine:{}", e))),
                        _ => {}
                    };
                }
            }
            None => {}
        };
    }

    true
}

fn loadPNG(png: &[u8]) -> Result<engine::Project, String> {
    use png::ColorType::*;
    let mut decoder = png::Decoder::new(&png[..]);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = match decoder.read_info() {
        Ok(r) => r,
        Err(err) => return Err(format!("decode png failed:{}", err)),
    };
    let mut img_data = vec![0; reader.output_buffer_size()];
    println!("png {} x {}", reader.info().width, reader.info().height);
    let info = match reader.next_frame(&mut img_data) {
        Ok(i) => i,
        Err(err) => return Err(format!("decode frame failed:{}", err)),
    };
    assert_eq!(info.color_type, Rgba);
    let d = &img_data[..];
    //https://github.com/microsoft/pxt/blob/master/pxtlib/util.ts decodeBlobAsync
    let bpp = (d[0] & 1) | ((d[1] & 1) << 1) | ((d[2] & 1) << 2);
    if bpp > 5 || bpp == 0 {
        return Err(String::from("Invalid encoded PNG format"));
    }
    let decode = |mut ptr: usize, bpp: u8, tgr_len: usize| -> (usize, Vec<u8>) {
        let mut shift = 0u8;
        let mut i = 0usize;
        let mut acc = 0u8;
        let mask = (1 << bpp) - 1;
        let mut tgr = vec![];
        while i < tgr_len {
            acc |= (d[ptr] & mask) << shift;
            ptr += 1;
            if ptr & 3 == 3 {
                ptr += 1;
            }
            shift += bpp;
            if shift >= 8 {
                tgr.push(acc & 0xff);
                i += 1;
                acc = 0;
                shift -= 8;
            }
        }
        (ptr, tgr)
    };

    let IMG_HEADER_SIZE = 36;
    let (ptr, hd) = decode(4, bpp, IMG_HEADER_SIZE);
    let (_, dhd, _) = unsafe { (&hd[..]).align_to::<u32>() };
    if dhd[0] != 0x59347a7d {
        return Err(String::from("Invalid magic in encoded PNG"));
    }
    let ret_len = dhd[1] as usize;
    let added_lines = dhd[2];
    let res = if added_lines > 0 {
        let orig_size = (reader.info().height - added_lines) * reader.info().width;
        let img_cap = (orig_size - 1) * 3 * bpp as u32 >> 3;
        let (_, mut res) = decode(ptr, bpp, img_cap as usize - IMG_HEADER_SIZE);
        let added = decode(orig_size as usize * 4, 8, ret_len - res.len());
        res.extend(added.1);
        res
    } else {
        let (_, res) = decode(ptr, bpp, ret_len);
        res
    };
    let content = if res[0] == b'{' {
        String::from_utf8(res).unwrap()
    } else {
        let mut decoded: Vec<u8> = vec![];
        let mut sl = &res[..];
        if lzma_rs::lzma_decompress(&mut sl, &mut decoded).is_err() {
            return Err(String::from("lzma decompress failed"));
        }
        String::from_utf8(decoded).unwrap()
    };
    let obj: serde_json::Value = serde_json::from_str(content.as_str()).unwrap();
    let source_json = obj
        .as_object()
        .unwrap()
        .get("source")
        .unwrap()
        .as_str()
        .unwrap();
    let source_obj: serde_json::Value = serde_json::from_str(source_json).unwrap();

    let mut prj = engine::Project {
        sources: HashMap::new(),
    };
    prj.sources.insert(
        String::from("main.ts"),
        ts2js("main.ts",source_obj.get("main.ts").unwrap().as_str().unwrap()),
    );

    //println!("{}", source_obj.get("main.ts").unwrap().as_str().unwrap());

    //std::fs::File::create(r"C:\Users\YDJiang\Downloads\ffmpeg-4.4-full_build\bin\bmp.raw").unwrap().write_all(&img_data[..]);
    //fmpeg.exe -f rawvideo  -pixel_format rgba -y -s 512x424  -i bmp.rgba -frames:v 1 image.png
    Ok(prj)
}

#[test]
fn test_ts2js(){
    let mut src = "".to_string();
    let read_rst = File::open("test.ts").unwrap().read_to_string(&mut src);
    assert!(read_rst.is_ok());
    println!("final js:{}", ts2js("main.ts", src.as_str()));
}

struct WrapVecWr{
    v:Vec<u8>,
}

impl std::io::Write for WrapVecWr {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        return self.v.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::Result::Ok(())
    }
}

fn ts2js(name: &str, src: &str) -> String {
    use swc::{config::Options, common::{errors::Handler, SourceMap,FileName, sync::Lrc}, ecmascript::ast::EsVersion};
    use swc_ecma_parser::Syntax;
    use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
    use swc_error_reporters::{PrettyEmitter, PrettyEmitterConfig, GraphicalReportHandler};

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom(name.into()), src.into());    
    let mut opt:Options = Default::default();
    opt.config.jsc.syntax = Some(Syntax::Typescript(Default::default()));
    opt.config.jsc.target = Some(EsVersion::Es2022);
    //let handler = Handler::with_emitter(can_emit_warnings, treat_err_as_bug, emitter)
    //compiler.process_js(handler, program, opts)
    let error_wr:Box<String>= Box::new(String::from(""));
    let mut dest_wr:Vec<u8> = vec![];
    {
        let emitter = PrettyEmitter::new(
            cm.clone(),
            error_wr,
            GraphicalReportHandler::default(),
            PrettyEmitterConfig {
                skip_filename: false
            },
        );
        let handler = Handler::with_emitter(true, false, Box::new(emitter));
        let _emitter = Emitter {
            cfg: Default::default(),
            cm:cm.clone(),
            wr: Box::new(JsWriter::new(
                cm.clone(),
                "\n",
                &mut dest_wr,
                None,
            )),
            comments: None, //Some(comments),
        };
        let compiler = swc::Compiler::new(cm);
        match compiler.process_js_file(fm, &handler, &opt) {
            Ok(out)=>out.code,
            Err(e)=>e.to_string(),
        }
    }
}

#[no_mangle]
extern "C" fn retro_set_environment(cb: bindings::retro_environment_t) {
    D("retro_set_environment()");
    unsafe {
        ENVIRON_CB = cb;
        //TODO: makecode arcade saved .PNG file as ROM
        // let mut no_rom = true;
        // cb.unwrap()(
        //     bindings::RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME,
        //     std::ptr::addr_of_mut!(no_rom) as *mut c_void,
        // );
    }
}

#[no_mangle]
extern "C" fn retro_set_video_refresh(cb: bindings::retro_video_refresh_t) {
    D("retro_set_video_refresh()");
    unsafe {
        VIDEO_CB = cb;
    }
}

#[no_mangle]
extern "C" fn retro_set_audio_sample(cb: bindings::retro_audio_sample_t) {
    D("retro_set_audio_sample()");
    unsafe {
        AUDIO_CB = cb;
    }
}

#[no_mangle]
extern "C" fn retro_set_input_poll(cb: bindings::retro_input_poll_t) {
    D("retro_set_input_poll()");
    unsafe {
        INPUT_POLL_CB = cb;
    }
}

#[no_mangle]
extern "C" fn retro_set_input_state(cb: bindings::retro_input_state_t) {
    D("retro_set_input_state()");
    unsafe {
        INPUT_STATE_CB = cb;
    }
}

static mut ENGINE: Option<engine::Engine> = None;

#[no_mangle]
extern "C" fn retro_init() {
    D("retro_init()");
    let mut log = bindings::retro_log_callback { log: None };
    unsafe {
        INPUT_STATES = Some(HashMap::new());
        ENVIRON_CB.unwrap()(
            bindings::RETRO_ENVIRONMENT_GET_LOG_INTERFACE,
            ptr::addr_of_mut!(log) as *mut c_void,
        );
        LOG_CB = log.log;
    }
    unsafe {
        ENGINE = Some(engine::Engine::new(FPS, AUDIO_HZ));
    }
}

#[no_mangle]
extern "C" fn retro_get_system_info(info: *mut bindings::retro_system_info) {
    D("retro_get_system_info()");
    unsafe {
        ptr::write_bytes(info, 0, 1);
        (*info).library_name = static_cptr!("makecode-arcade");
        (*info).library_version = static_cptr!("0.1.3");
        (*info).need_fullpath = false;
        (*info).block_extract = false;
        (*info).valid_extensions = static_cptr!("png");
    }
}

#[no_mangle]
extern "C" fn retro_get_region() -> u32 {
    D("retro_get_region()");
    bindings::RETRO_REGION_PAL //NTSC
}

#[no_mangle]
extern "C" fn retro_get_system_av_info(info: *mut bindings::retro_system_av_info) {
    D("retro_get_system_av_info()");
    unsafe {
        ptr::write_bytes(info, 0, 1);
        (*info).timing.fps = FPS as f64;
        (*info).timing.sample_rate = AUDIO_HZ as f64;
        (*info).geometry.base_width = emulator::game::BMP_WIDTH;
        (*info).geometry.base_height = emulator::game::BMP_HEIGHT;
        (*info).geometry.max_width = emulator::game::BMP_WIDTH;
        (*info).geometry.max_height = emulator::game::BMP_HEIGHT;
        (*info).geometry.aspect_ratio =
            emulator::game::BMP_WIDTH as f32 / emulator::game::BMP_HEIGHT as f32;
    }
}

#[no_mangle]
extern "C" fn retro_reset() {
    D("retro_reset");
    unsafe {
        match ENGINE
            .as_ref()
            .unwrap()
            .event_tx
            .send(engine::Event::Unload)
        {
            Err(e) => I(tmp_c_str(format!("send unload to engine:{}", e))),
            _ => {}
        };
    }
}

static mut TEN_CC: i32 = 20;

#[no_mangle]
extern "C" fn retro_run() {
    // one step(aka. one frame) of game.
    // 1. get the previous rendered frame buffer to this thread
    // 2. move one step(20ms) forward in game engine working thread
    unsafe {
        //VIDEO_CB.unwrap()(FRAME_BUFFER.as_ptr() as *const c_void, emulator::game::BMP_WIDTH, emulator::game::BMP_HEIGHT, emulator::game::BMP_WIDTH * 4);
        if TEN_CC > 0 {
            I(tmp_c_str(format!(
                "{:?}:retro_run()",
                std::time::Instant::now()
            )));
            TEN_CC -= 1;
        }
    }

    unsafe {
        INPUT_POLL_CB.unwrap()();
    }

    unsafe {
        let state_cb = INPUT_STATE_CB.unwrap();
        for id in [
            (bindings::RETRO_DEVICE_ID_JOYPAD_LEFT, KeyCode::Left),
            (bindings::RETRO_DEVICE_ID_JOYPAD_RIGHT, KeyCode::Right),
            (bindings::RETRO_DEVICE_ID_JOYPAD_UP, KeyCode::Up),
            (bindings::RETRO_DEVICE_ID_JOYPAD_DOWN, KeyCode::Down),
            (bindings::RETRO_DEVICE_ID_JOYPAD_A, KeyCode::A),
            (bindings::RETRO_DEVICE_ID_JOYPAD_B, KeyCode::B),
            (bindings::RETRO_DEVICE_ID_JOYPAD_X, KeyCode::X),
            (bindings::RETRO_DEVICE_ID_JOYPAD_Y, KeyCode::Y),
        ] {
            let old_pressed = match INPUT_STATES.as_ref().unwrap().get(&id.0) {
                Some(v) => *v,
                _ => false,
            };
            let new_pressed = state_cb(0, bindings::RETRO_DEVICE_JOYPAD, 0, id.0) != 0;
            INPUT_STATES.as_mut().unwrap().insert(id.0, new_pressed);
            //TODO: key repeat
            if old_pressed && !new_pressed {
                I(tmp_c_str(format!("UP {}", id.1)));
                ENGINE
                    .as_ref()
                    .unwrap()
                    .event_tx
                    .send(engine::Event::KeyUp(id.1))
                    .unwrap_or_else(|e| {
                        I(tmp_c_str(format!("send key event to engine:{e}")));
                    })
            }
            if !old_pressed && new_pressed {
                I(tmp_c_str(format!("DOWN {}", id.1)));
                ENGINE
                    .as_ref()
                    .unwrap()
                    .event_tx
                    .send(engine::Event::KeyDown(id.1))
                    .unwrap_or_else(|e| {
                        I(tmp_c_str(format!("send key event to engine:{e}")));
                    })
            }
        }
    }

    match unsafe { ENGINE.as_ref().unwrap() }.fb_rx.recv() {
        Err(e) => I(tmp_c_str(format!("receive fb error:{}", e))),
        Ok(fb) => {
            let fbp = fb.as_ptr();
            unsafe {
                VIDEO_CB.unwrap()(
                    fbp as *const c_void,
                    emulator::game::BMP_WIDTH,
                    emulator::game::BMP_HEIGHT,
                    (emulator::game::BMP_WIDTH * 4) as bindings::size_t,
                );
            }
        }
    }
}

#[no_mangle]
extern "C" fn retro_cheat_reset() {
    D("retro_cheat_reset()");
}

#[no_mangle]
extern "C" fn retro_cheat_set(_index: u32, _enabled: bool, _ptr: *const c_char) {
    D("retro_cheat_set()");
}

#[no_mangle]
extern "C" fn retro_load_game_special(
    _game_type: u32,
    _info: *const bindings::retro_game_info,
    _num_info: usize,
) -> bool {
    D("retro_load_game_special()");
    false
}

#[no_mangle]
extern "C" fn retro_set_controller_port_device(_port: u32, _device: u32) {
    D("retro_set_controller_port_device()")
}
#[no_mangle]
extern "C" fn retro_get_memory_data(_id: u32) -> *const c_void {
    D("retro_get_memory_data()");
    ptr::null()
}
#[no_mangle]
extern "C" fn retro_get_memory_size(_id: u32) -> usize {
    D("retro_get_memory_size()");
    0
}

#[no_mangle]
extern "C" fn retro_serialize_size() -> usize {
    D("retro_serialize_size()");
    0
}

#[no_mangle]
extern "C" fn retro_serialize(_data: *const c_void, _size: usize) -> bool {
    D("retro_serialize()");
    false
}

#[no_mangle]
extern "C" fn retro_unserialize(_data: *mut c_void, _size: usize) -> bool {
    D("retro_unserialize()");
    false
}

#[no_mangle]
extern "C" fn retro_deinit() {
    D("retro_deinit()");
    unsafe {
        match ENGINE.as_ref().unwrap().event_tx.send(engine::Event::Exit) {
            Err(e) => D(tmp_c_str(format!("send tick to engine:{}", e))),
            Ok(_) => {}
        };
        ENGINE = None;
        //LOG_CB = None; //log can work even deinit()-ed.
    }
}
#[no_mangle]
extern "C" fn retro_set_audio_sample_batch(_cb: bindings::retro_audio_sample_batch_t) {
    D("retro_set_audio_sample_batch()");
}
