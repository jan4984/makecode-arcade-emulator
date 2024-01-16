use core::time;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use v8::Local;

//scene lifecycle managed by rust. it's auto created at game start/restart
//sprite lifecycle managed by rust. because it is added to scene after creating. refer by usize from js
//js bitmap is just a js string. after set to scene/sprite/animation, bitmap data(color\width\height) is cloned, not referenced, so the bitmap in js may GCed.
//animation in rust side. refer by usize from js
use crate::{
    emulator::{
        self,
        game::{BMP_HEIGHT, BMP_WIDTH},
        resource::Bitmap,
    },
    engine::{self, Event},
};

//struct DROP in filed declare order
pub struct Runtime {}

static V8_INIT: std::sync::Once = std::sync::Once::new();
static mut V8_ISOLATE: Option<v8::OwnedIsolate> = None;
static mut V8_CONTEXT_TMPL: Option<Local<v8::ObjectTemplate>> = None;
static mut V8_TOP_SCOPE: Option<v8::HandleScope<'static, ()>> = None;
static mut V8_CONTEXT: Option<Local<v8::Context>> = None;
static mut V8_CONTEXT_SCOPE: Option<v8::ContextScope<'static, v8::HandleScope<'static>>> = None;
//TODO: bind SCENE\INFO to a global v8 object?
static mut SCENE: Option<emulator::scene::Scene> = None;
static mut INFO: Option<emulator::info::Info> = None;
static mut GAME: Option<emulator::game::Game> = None;
static BINDING_SRC: &[u8]=std::include_bytes!("binding.js");

macro_rules! add_fn {
    ($obj:ident, $fn:ident) => {
        $obj.set(
            v8::String::new(V8_TOP_SCOPE.as_mut().unwrap(), stringify!($fn))
                .unwrap()
                .into(),
            v8::FunctionTemplate::new(V8_TOP_SCOPE.as_mut().unwrap(), $fn).into(),
        );
    };
}

/*
module.set(
    v8::String::new(V8_TOP_SCOPE.as_mut().unwrap(), "sprite_set_x")
        .unwrap()
        .into(),
    v8::FunctionTemplate::new(V8_TOP_SCOPE.as_mut().unwrap(), |scope: &mut v8::HandleScope, args: v8::FunctionCallbackArguments, mut _retval: v8::ReturnValue|{
        let idx = args.get(0).int32_value(scope).unwrap() as usize;
        let value = args.get(1).number_value(scope).unwrap() as f32;
        let sprite = SCENE.as_mut().unwrap().get_mut(idx);
        sprite.x = value.into();
    }).into(),
);
*/

macro_rules! add_sprite_prority {
    ($obj:ident, $p:ident) => {
        $obj.set(
            v8::String::new(
                V8_TOP_SCOPE.as_mut().unwrap(),
                std::concat!("sprite_set_", stringify!($p)),
            )
            .unwrap()
            .into(),
            v8::FunctionTemplate::new(
                V8_TOP_SCOPE.as_mut().unwrap(),
                |scope: &mut v8::HandleScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let idx = args.get(0).int32_value(scope).unwrap() as usize;
                    let value = args.get(1).number_value(scope).unwrap() as f32;
                    println!("set property {} of sprite {idx} to {value}", stringify!($p));
                    if value == f32::NAN {
                        //TODO: throw exception
                        return;
                    }
                    let sprite = SCENE.as_mut().unwrap().get_mut(idx);
                    sprite.$p = value.into();
                },
            )
            .into(),
        );

        $obj.set(
            v8::String::new(
                V8_TOP_SCOPE.as_mut().unwrap(),
                std::concat!("sprite_get_", stringify!($p)),
            )
            .unwrap()
            .into(),
            v8::FunctionTemplate::new(
                V8_TOP_SCOPE.as_mut().unwrap(),
                |scope: &mut v8::HandleScope,
                 args: v8::FunctionCallbackArguments,
                 mut _retval: v8::ReturnValue| {
                    let idx = args.get(0).int32_value(scope).unwrap() as usize;
                    let sprite = SCENE.as_ref().unwrap().get(idx);
                    _retval.set(v8::Number::new(scope, sprite.$p as f64).into());
                },
            )
            .into(),
        );
    };
}

impl Drop for Runtime {
    fn drop(&mut self) {
        println!("drop engine Runtime()");
        unsafe {
            SCENE = None;
            V8_CONTEXT_SCOPE = None;
            V8_CONTEXT = None;
        }
    }
}

fn v8_get_global<'s>(name: &str) -> v8::Local<'s, v8::Value> {
    unsafe {
        let scope = V8_CONTEXT_SCOPE.as_mut().unwrap();
        match V8_CONTEXT.as_ref().unwrap().global(scope).get(
            V8_CONTEXT_SCOPE.as_mut().unwrap(),
            v8::String::new(V8_CONTEXT_SCOPE.as_mut().unwrap(), name)
                .unwrap()
                .into(),
        ) {
            None => v8::undefined(scope).into(),
            Some(v) => v,
        }
    }
}

fn v8_get_string(scope: &mut v8::HandleScope, obj: v8::Local<v8::Value>) -> String {
    if !obj.is_string() {
        String::from("")
    } else {
        obj.to_string(scope).unwrap().to_rust_string_lossy(scope)
    }
}

fn v8_get_i32(scope: &mut v8::HandleScope, obj: v8::Local<v8::Value>) -> i32 {
    if !obj.is_number() {
        return 0;
    } else {
        obj.to_int32(scope).unwrap().int32_value(scope).unwrap()
    }
}

impl Runtime {
    fn reset_context(&mut self) {
        unsafe {
            //drop in reverse order.or panic
            V8_CONTEXT_SCOPE = None;

            let context_ = v8::Context::new_from_template(
                V8_TOP_SCOPE.as_mut().unwrap(),
                V8_CONTEXT_TMPL.unwrap(),
            );
            V8_CONTEXT = Some(context_);
            let context = V8_CONTEXT.as_mut().unwrap();

            let context_scope_ = v8::ContextScope::new(V8_TOP_SCOPE.as_mut().unwrap(), *context);
            V8_CONTEXT_SCOPE = Some(context_scope_);
        }
    }

    pub fn process_events(&self, evt: engine::Event) {
        let msg = match evt {
            Event::KeyDown(_) | Event::KeyUp(_) | Event::KeyRepeat(_) => evt.to_string(),
            _ => {
                return;
            }
        };

        let cb = v8_get_global("_221149842913key_events_cb");
        if cb.is_function() {
            let loop_cb = v8::Local::<v8::Function>::try_from(cb).unwrap();
            let mut scope = v8::HandleScope::new(unsafe { V8_CONTEXT_SCOPE.as_mut().unwrap() });
            let mut try_catch = v8::TryCatch::new(&mut scope);
            let udf: v8::Local<v8::Value> = v8::undefined(&mut try_catch).into();
            let msg_str = v8::String::new(&mut try_catch, msg.as_str()).unwrap();
            match loop_cb.call(&mut try_catch, udf, &[msg_str.into()]) {
                None => {
                    println!("{}", report_exceptions(&mut try_catch));
                }
                _ => {}
            };
        }
    }

    pub fn draw(&self, canvas: &mut emulator::resource::Canvas) {
        unsafe {
            SCENE.as_ref().unwrap().draw(canvas);
            INFO.as_ref().unwrap().draw(canvas);
        }
    }

    pub fn update(&mut self, micro_sec: u64) {
        if unsafe { GAME.as_ref().unwrap().over } {
            return;
        }
        {
            let cb = v8_get_global("_221149842913game_loop");
            if cb.is_function() {
                let loop_cb = v8::Local::<v8::Function>::try_from(cb).unwrap();
                let mut scope = v8::HandleScope::new(unsafe { V8_CONTEXT_SCOPE.as_mut().unwrap() });
                let mut try_catch = v8::TryCatch::new(&mut scope);
                let udf: v8::Local<v8::Value> = v8::undefined(&mut try_catch).into();
                let dt = v8::Number::new(&mut try_catch, (micro_sec / 1000 ) as f64);
                match loop_cb.call(&mut try_catch, udf, &[dt.into()]) {
                    None => {
                        println!("{}", report_exceptions(&mut try_catch));
                    }
                    _ => {}
                };
            }
        };

        unsafe {
            let dt = time::Duration::from_millis(micro_sec / 1000);
            SCENE.as_mut().unwrap().update(&dt);
        }
    }

    pub fn reset(&mut self) {
        unsafe {
            SCENE = None;
            INFO = None;
            GAME = None;
        }

        self.reset_context();
        //TODO: load binding.js
        let scene =
            emulator::scene::Scene::new(Rc::new(RefCell::new(emulator::resource::FrameMgr {
                frames: HashMap::new(),
            })));
        unsafe {
            GAME = Some(emulator::game::Game::new());
            INFO = Some(emulator::info::Info::new());
            SCENE = Some(scene);            
        }
        self.run_script(&String::from(std::str::from_utf8(BINDING_SRC).unwrap()));
    }

    pub fn process_overlap_check(&self) {
        let overlaps = unsafe { SCENE.as_ref().unwrap().check_overlaps() };
        if overlaps.len() == 0 {
            return;
        }
        let overlap_cb = {
            let cb = v8_get_global("_221149842913overlap_cb");
            if cb.is_function() {
                v8::Local::<v8::Function>::try_from(cb).unwrap()
            } else {
                return;
            }
        };

        let mut scope = v8::HandleScope::new(unsafe { V8_CONTEXT_SCOPE.as_mut().unwrap() });
        let mut try_catch = v8::TryCatch::new(&mut scope);
        let udf: v8::Local<v8::Value> = v8::undefined(&mut try_catch).into();
        for overlap in overlaps {
            let mut args: Vec<v8::Local<v8::Value>> = vec![];
            for v in [overlap.0, overlap.1, overlap.2, overlap.3].iter() {
                args.push(v8::Integer::new_from_unsigned(&mut try_catch, *v as u32).into());
            }
            match overlap_cb.call(&mut try_catch, udf, &args) {
                None => {
                    println!("{}", report_exceptions(&mut try_catch));
                }
                _ => {}
            };
        }
    }

    pub fn run_script(&self, script_content: &String) {
        unsafe {
            let script =
                v8::String::new(V8_CONTEXT_SCOPE.as_mut().unwrap(), script_content.as_str())
                    .unwrap();
            let mut scope = v8::HandleScope::new(V8_CONTEXT_SCOPE.as_mut().unwrap());
            let mut try_catch = v8::TryCatch::new(&mut scope);
            let script = match v8::Script::compile(&mut try_catch, script, None) {
                Some(s) => s,
                None => {
                    println!("compile failed!");
                    println!("{}", report_exceptions(&mut try_catch));
                    return;
                }
            };

            match script.run(&mut try_catch) {
                None => {
                    println!("run failed!");
                    println!("{}", report_exceptions(&mut try_catch));
                    return;
                }
                Some(_mod) => (),
            }
        }
    }

    pub fn new() -> Self {
        V8_INIT.call_once(|| {
            let platform = v8::new_default_platform(0, false).make_shared();
            v8::V8::initialize_platform(platform);
            v8::V8::initialize();

            unsafe {
                let isolate_ = v8::Isolate::new(v8::CreateParams::default());
                V8_ISOLATE = Some(isolate_);
                let isolate = V8_ISOLATE.as_mut().unwrap();

                let top_scope_ = v8::HandleScope::new(isolate);
                V8_TOP_SCOPE = Some(top_scope_);
                let top_scope = V8_TOP_SCOPE.as_mut().unwrap();

                let module = v8::ObjectTemplate::new(top_scope);
                add_fn!(module, scene_set_effect);
                add_fn!(module, scene_set_background_color);
                add_fn!(module, scene_add_sprite);
                add_fn!(module, scene_add_overlap_check_kinds);

                add_fn!(module, sprite_kind_create);

                add_fn!(module, animation_add);
                add_fn!(module, animation_add_frame);

                add_fn!(module, info_set_score);
                add_fn!(module, info_change_score);

                add_fn!(module, sprite_active_action);
                add_fn!(module, sprite_set_flag);
                add_fn!(module, sprite_set_bound);
                add_fn!(module, sprite_get_bound);

                add_fn!(module, game_over);

                add_sprite_prority!(module, x);
                add_sprite_prority!(module, y);
                add_sprite_prority!(module, ax);
                add_sprite_prority!(module, ay);
                add_sprite_prority!(module, vx);
                add_sprite_prority!(module, vy);
                add_sprite_prority!(module, fx);
                add_sprite_prority!(module, fy);
                add_sprite_prority!(module, sx);
                add_sprite_prority!(module, sy);

                let global = v8::ObjectTemplate::new(top_scope);
                global.set(
                    v8::String::new(top_scope, "_engine").unwrap().into(),
                    module.into(),
                );
                global.set(
                    v8::String::new(top_scope, "_log").unwrap().into(),
                    v8::FunctionTemplate::new(V8_TOP_SCOPE.as_mut().unwrap(), js_Log).into(),
                );
                V8_CONTEXT_TMPL = Some(global);

                // let context_ = v8::Context::new_from_template(V8_TOP_SCOPE.as_mut().unwrap(), V8_CONTEXT_TMPL.unwrap());
                // V8_CONTEXT = Some(context_);
            }
        });

        let mut self_ = Runtime {};
        self_.reset();
        self_
    }
}

fn scene_add_overlap_check_kinds(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }

    if !args.get(0).is_uint32() || !args.get(1).is_uint32() {
        return;
    }
    let (kind1, kind2) = (
        v8_get_i32(scope, args.get(0)) as u32,
        v8_get_i32(scope, args.get(1)) as u32,
    );
    unsafe { SCENE.as_mut().unwrap() }.add_overlap_detection(kind1 as usize, kind2 as usize);
}
fn scene_add_sprite(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }

    if !args.get(0).is_string() || !args.get(1).is_number() {
        return;
    }

    let bmp_data = v8_get_string(scope, args.get(0));
    let kind = v8_get_i32(scope, args.get(1)) as usize;
    let bmp = emulator::resource::Bitmap::new_from_string_without_size(bmp_data.as_str());
    let sprite = emulator::sprite::Sprite::new_with_bmp(bmp, kind);
    unsafe {
        let sprite_ref = SCENE.as_mut().unwrap().add_sprite(sprite);
        _retval.set(v8::Integer::new(scope, sprite_ref as i32).into());
    }
}

fn animation_add(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }
    let name = v8_get_string(scope, args.get(0));
    if name == "" {
        return;
    }
    let interval = v8_get_i32(scope, args.get(1)) as u32;
    if interval == 0 {
        return;
    }

    unsafe { SCENE.as_ref().unwrap() }
        .frame_mgr
        .borrow_mut()
        .create(name.as_str(), interval);
}

fn animation_add_frame(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }
    let name = v8_get_string(scope, args.get(0));
    if name == "" {
        return;
    }
    let frame = v8_get_string(scope, args.get(1));
    //println!("to add frame to animation {}", name);
    unsafe { SCENE.as_ref().unwrap() }
        .frame_mgr
        .borrow_mut()
        .append(
            name.as_str(),
            Bitmap::new_from_string_without_size(frame.as_str()),
        );
}

fn sprite_active_action(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    let idx = args.get(0).int32_value(scope).unwrap() as usize;
    let animation_name = v8_get_string(scope, args.get(1));
    let sprite = unsafe { SCENE.as_mut().unwrap() }.get_mut(idx);
    sprite.active_animation(animation_name);
}

fn sprite_set_flag(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() < 3 {
        return;
    }

    if !args.get(0).is_number() || !args.get(1).is_number() || !args.get(2).is_number() {
        return;
    }

    let idx = args.get(0).int32_value(scope).unwrap() as usize;
    let flag = args.get(1).uint32_value(scope).unwrap() as u32;
    let true_false = args.get(1).int32_value(scope).unwrap();
    let sprite = unsafe { SCENE.as_mut().unwrap() }.get_mut(idx);
    sprite.set_flag(flag, true_false != 0);
}

fn game_over(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() == 0 {
        return;
    }

    if !args.get(0).is_number() {
        return;
    }

    let game = unsafe { GAME.as_mut().unwrap() };
    game.over = true;
    game.win = v8_get_i32(scope, args.get(0)) == 1;
}

fn sprite_set_bound(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() < 3 {
        return;
    }

    if !args.get(0).is_number() || !args.get(1).is_string() || !args.get(2).is_number() {
        return;
    }
    let idx = args.get(0).int32_value(scope).unwrap() as usize;
    let side = v8_get_string(scope, args.get(1));
    let value = args.get(2).number_value(scope).unwrap() as f32;
    let sprite = unsafe { SCENE.as_mut().unwrap() }.get_mut(idx);
    match side.as_str() {
        "top" => {
            sprite.set_top(value as i32);
        }
        "bottom" => {
            sprite.set_bottom(value as i32);
        }
        "left" => {
            sprite.set_left(value as i32);
        }
        "right" => {
            sprite.set_right(value as i32);
        }
        _ => {}
    }
}

fn sprite_get_bound(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        return;
    }

    if !args.get(0).is_number() || !args.get(1).is_string() {
        return;
    }
    let idx = args.get(0).int32_value(scope).unwrap() as usize;
    let side = v8_get_string(scope, args.get(1));
    let sprite = unsafe { SCENE.as_mut().unwrap() }.get_mut(idx);
    let value = match side.as_str() {
        "top" => sprite.top(),
        "bottom" => sprite.bottom(),
        "left" => sprite.left(),
        "right" => sprite.right(),
        _ => {return;}
    };
    _retval.set(v8::Integer::new(scope, value).into());
}

fn sprite_kind_create(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    _retval.set(v8::Integer::new(scope, emulator::sprite::sprite_kind::create() as i32).into());
}

fn info_set_score(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() == 0 || !args.get(0).is_int32() {
        return;
    }

    unsafe { INFO.as_mut().unwrap() }.set_score(v8_get_i32(scope, args.get(0)) as f32);
}

fn info_change_score(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() == 0 || !args.get(0).is_number() {
        println!("info_change_score wrong arg");
        return;
    }

    let v = v8_get_i32(scope, args.get(0));
    //println!("change score delta {}", v);
    unsafe { INFO.as_mut().unwrap() }.change_score( v as f32);
}

fn js_Log(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() == 0 || !args.get(0).is_string() {
        return;
    }
    println!("{}", v8_get_string(scope, args.get(0)));
}

fn scene_set_effect(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() == 0 || !args.get(0).is_string() {
        return;
    }

    let str = v8_get_string(scope, args.get(0));
    let eff = match str.as_str() {
        "blizzard" => emulator::effect::SceneEffect::blizzard(),
        _ => {
            return;
        }
    };
    unsafe {
        SCENE.as_mut().unwrap().active_effect(eff);
    }
}

fn scene_set_background_color(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    if args.length() == 0 || !args.get(0).is_string() {
        return;
    }

    let str = v8_get_string(scope, args.get(0));
    if str.len() == 0 {
        return;
    }

    let bg = Bitmap::new_with_color(
        BMP_WIDTH as usize,
        BMP_HEIGHT as usize,
        str.bytes().next().unwrap(),
    );
    unsafe { SCENE.as_mut().unwrap().set_bgi(bg) }
}

#[test]
fn test_overlap_with_js() {
    let runtime = Runtime::new();
    runtime.run_script(&String::from(
        r"
(function(THIZ){
let k2=SpriteKind.create();
let sp1 =sprites.create(img`
    ..........
    .........a
    .........a
    bbbbbbbbbb
`, k2);
let sp2 = sprites.create(img`
    ..........
    ..........
    b.........
    ..........   
`, SpriteKind.Player);
sprites.onOverlap(SpriteKind.Player, k2, function(player, sp){
    THIZ._test_result=`${sp2==player&&sp==sp1}`;
});
//THIZ.sp1=sp1;
THIZ.sp2=sp2;
})(globalThis);
",
    ));
    runtime.process_overlap_check();
    assert!(v8_get_global("_test_result").is_undefined());
    runtime.run_script(&String::from(
        r"sp2.y=sp2.y+1;",
    ));
    runtime.process_overlap_check();
    let result = v8_get_global("_test_result");
    assert!(result.is_string());
    let str = unsafe {
        result
            .to_string(V8_CONTEXT_SCOPE.as_mut().unwrap())
            .unwrap()
            .to_rust_string_lossy(V8_CONTEXT_SCOPE.as_mut().unwrap())
    };
    assert_eq!(String::from(str), "true");
}

#[test]
fn test_runtime() {
    let mut runtime = Runtime::new();
    let mut canvas = emulator::resource::Canvas::new();
    runtime.run_script(&String::from("_engine.scene_set_background_color('b')"));
    unsafe {
        let scene = SCENE.as_mut().unwrap();
        scene.draw(&mut canvas);
        assert_eq!(canvas.get_pixel(13, 22), emulator::resource::COLORS[0xb]);
    }
    runtime.reset();
    runtime.run_script(&String::from("_engine.scene_set_background_color('5')"));
    unsafe {
        let scene = SCENE.as_mut().unwrap();
        scene.draw(&mut canvas);
        assert_eq!(canvas.get_pixel(159, 44), emulator::resource::COLORS[0x5]);
    }
}

fn report_exceptions(try_catch: &mut v8::TryCatch<v8::HandleScope>) -> String {
    let mut ret: Vec<u8> = vec![];
    let exception = try_catch.exception().unwrap();
    let exception_string = exception
        .to_string(try_catch)
        .unwrap()
        .to_rust_string_lossy(try_catch);
    let message = if let Some(message) = try_catch.message() {
        message
    } else {
        ret.extend(format!("{}\n", exception_string).as_bytes());
        return String::from_utf8(ret).unwrap();
    };

    // Print (filename):(line number): (message).
    let filename = message.get_script_resource_name(try_catch).map_or_else(
        || "(unknown)".into(),
        |s| {
            s.to_string(try_catch)
                .unwrap()
                .to_rust_string_lossy(try_catch)
        },
    );
    let line_number = message.get_line_number(try_catch).unwrap_or_default();

    ret.extend(format!("{}:{}: {}\n", filename, line_number, exception_string).as_bytes());

    // Print line of source code.
    let source_line = message
        .get_source_line(try_catch)
        .map(|s| {
            s.to_string(try_catch)
                .unwrap()
                .to_rust_string_lossy(try_catch)
        })
        .unwrap();
    ret.extend(format!("{}\n", source_line).as_bytes());

    // Print wavy underline (GetUnderline is deprecated).
    let start_column = message.get_start_column();
    let end_column = message.get_end_column();

    for _ in 0..start_column {
        ret.push(b' ');
    }

    for _ in start_column..end_column {
        ret.push(b'^');
    }

    ret.push(b'\n');

    // Print stack trace
    let stack_trace = if let Some(stack_trace) = try_catch.stack_trace() {
        stack_trace
    } else {
        return String::from_utf8(ret).unwrap();
    };
    let stack_trace = unsafe { v8::Local::<v8::String>::cast(stack_trace) };
    let stack_trace = stack_trace
        .to_string(try_catch)
        .map(|s| s.to_rust_string_lossy(try_catch));

    if let Some(stack_trace) = stack_trace {
        ret.extend(format!("{}", stack_trace).as_bytes());
    }
    String::from_utf8(ret).unwrap()
}
