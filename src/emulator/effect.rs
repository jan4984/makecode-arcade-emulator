use std::cell::RefCell;
use std::{collections::hash_map::DefaultHasher, time};
use std::hash::Hasher;

use crate::emulator::game::{BMP_HEIGHT, BMP_WIDTH};

use super::resource::{Bitmap, COLORS, Canvas, Rect};

pub trait EffectItem {
    fn new(left:i32, top:i32, width:usize, height:usize)->Self;
    fn renew(&self) -> Self;
    fn update(&mut self, dt: &time::Duration);
    fn get_bmp(&self) -> Option<(i32, i32, &Bitmap)>;
}


static COLOR_SNOW:u32=COLORS[1];

struct Snow {
    current: Bitmap,
    params:(i32,i32,usize,usize),
    x:f32,
    y:f32,
    life: time::Duration,
    age: time::Duration,
    speed_pixels_per_ms: f32,
}

thread_local! {
    static HASHER:RefCell<DefaultHasher> = RefCell::new(DefaultHasher::default());
}

pub fn rand() -> f32{
    let seed = time::SystemTime::now().duration_since(time::SystemTime::UNIX_EPOCH).unwrap().as_nanos();
    HASHER.with(|hh|{
        let mut h = hh.borrow_mut();
        h.write(&seed.to_le_bytes()[..]);
        h.finish() as f32 / u64::MAX as f32
    })
}

pub fn randint(min:i32, max:i32) -> i32 {
    ((rand() * ((max-min) as f32)) as i32) + min
}

impl EffectItem for Snow{
    fn renew(&self) -> Self{
        let(a,b,c,d)=self.params;
        Self::new(a,b,c,d)
    }
    fn new(left:i32, top:i32, width:usize, height:usize) -> Self{
        let x = (rand() * width as f32) as i32 + left;
        let y = (rand() * height as f32) as i32 + top;
        //0.5 ~ 1 sec
        let life = time::Duration::from_secs_f32((500.0 + rand() * 500.0)/1000.0);
        let mut speed_pixels_per_ms = width as f32 / life.as_millis() as f32;
        speed_pixels_per_ms = (rand() + 1.0) * speed_pixels_per_ms;
        let mut size = (rand() * 5 as f32) as i32;
        if size == 4 {
            size = 1;
        }

        let rect = Rect{x,y,w:if size == 3 {2}else{1},h:if size >= 2 {2} else {1}};
        let current = Bitmap::new_with_bmp(vec![COLOR_SNOW;(rect.w*rect.h) as usize], rect.w);
        Snow { speed_pixels_per_ms, age:time::Duration::from_millis(0), current, x:left as f32, y: top as f32, life, params:(left,top,width,height)}
    }

    fn update(&mut self, dt: &time::Duration){                
        self.age += *dt;
        let dice = rand();
        if dice <= 0.6 {
            //0.6 possibility move in self.speed_pixels_per_ms
            self.x += dt.as_millis() as f32 * self.speed_pixels_per_ms;
            self.y += dt.as_millis() as f32 * self.speed_pixels_per_ms;
        }else if dice <= 0.9 {
            //0.3 possibility move double speed
            self.x += dt.as_millis() as f32 * self.speed_pixels_per_ms * 2.0;
            self.y += dt.as_millis() as f32 * self.speed_pixels_per_ms * 2.0;
        }else{
            //0.1 possibility move triple speed
            self.x += dt.as_millis() as f32 * self.speed_pixels_per_ms * 3.0;
            self.y += dt.as_millis() as f32 * self.speed_pixels_per_ms * 3.0;
        }
    }

    fn get_bmp(& self) -> Option<(i32, i32, &Bitmap)> {
        if self.age >= self.life { None } else { Some((self.x as i32, self.y as i32, &self.current)) }
    }
}

struct EffectContainer<T:Sized + EffectItem> {
    items: Vec<T>,
}

pub trait Effect {
    fn update(&mut self, dt:&time::Duration);
    fn draw(&self, canvas: &mut Canvas);
}

impl<T:Sized + EffectItem> Effect for EffectContainer<T> {
    fn update(&mut self, dt:&time::Duration) {        
        for i in 0..self.items.len(){
            let item = self.items.get_mut(i).unwrap();
            item.update(dt);
            if item.get_bmp().is_none() {
                self.items[i] = item.renew();
            }
        }
    }
    
     fn draw(&self, canvas: &mut Canvas){
        for s in self.items.iter() {
            match s.get_bmp(){
                None=>{},
                Some((x, y, bmp))=>{
                    canvas.draw(x, y, bmp);
                }
            }
        }
    }
}

pub struct SceneEffect();

impl SceneEffect {
    pub fn blizzard() -> Box<dyn Effect>{
        let mut items : Vec<Snow> = vec![];
        let width = BMP_WIDTH as i32 / 5;
        let height = BMP_HEIGHT as i32 / 4;
        for x in (-(width as i32)..BMP_WIDTH as i32).step_by(width as usize) {
            for y in (-(height as i32)..BMP_HEIGHT as i32).step_by(height as usize) {
                items.push(Snow::new(x, y, width as usize, height as usize));
            }
        }
        Box::new(EffectContainer::<Snow>{items})
    }
    pub fn dummy() -> Box<dyn Effect> {
        Box::new(EffectContainer::<Snow>{items:vec![]})
    }
}
