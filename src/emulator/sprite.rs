use std::{time};

use super::{game::{BMP_HEIGHT, BMP_WIDTH}, resource::{Animation, Bitmap, FrameMgr, Rect}};

#[repr(u32)]
pub enum Flag{
    Autodestory=0x1u32,
    Invisible=0x2u32,
}

impl Flag {
    pub fn u32(self) -> u32 {
        self as u32
    }
}

pub struct Sprite{
    pub x:f32,
    pub y:f32,
    pub z:i32,
    pub z2:i32,//if z eq, z2 used for overlap detection
    pub vx:f32,
    pub vy:f32,
    pub ax:f32,
    pub ay:f32,
    pub fx:f32,
    pub fy:f32,
    pub sx:f32,
    pub sy:f32,
    pub kind:usize,    
    pub width:usize,
    pub height:usize,
    image:Bitmap,    
    anmi: Animation,
    pub flag: u32,
}

impl Sprite {
    pub fn new() -> Sprite{
        Sprite{flag:0u32,x:(BMP_WIDTH>>1) as f32,y:(BMP_HEIGHT>>1) as f32,z:0,z2:0,vx:0f32,vy:0f32,ax:0f32,ay:0f32,fx:0f32,fy:0f32,sx:0f32,sy:0f32,kind:0,width:0,height:0,image:Bitmap::new(0,0), anmi:Animation::new()}
    }
    pub fn new_with_bmp(bmp: Bitmap, kind: usize) -> Sprite{        
        let mut s = Sprite::new();
        s.image = bmp;
        s.width = s.image.width();
        s.height = s.image.height();
        s.kind = kind;
        //s.height = bmp.pixels.len()
        s
    }
    pub fn set_left(&mut self, x:i32) {
        self.x = (x + (self.width /2 ) as i32) as f32;
    }
    pub fn set_right(&mut self, x:i32) {
        self.x = (x - (self.width /2 )as i32) as f32;
    }
    pub fn set_top(&mut self, y:i32) {
        self.y = (y + (self.height / 2)as i32) as f32;
    }
    pub fn set_bottom(&mut self, y:i32) {
        self.y = (y - (self.height /2 )as i32) as f32;
    }
    pub fn set_flag(&mut self, flag: u32, on:bool){
        if on {
            self.flag = self.flag | flag;
        }else{
            self.flag = self.flag & !flag;
        }
    }
    pub fn current_image<'a>(&self, frame_mgr:&'a FrameMgr) -> Option<&'a Bitmap>{
        self.anmi.current(frame_mgr)
    }
    pub fn native_image(&self) -> &Bitmap{
        &self.image
    }
    pub fn attach_animation(&mut self, _name:String){
        //global refer by name, attach doing nothing 
        //self.anmi.reset(name);
    }
    pub fn active_animation(&mut self, name:String){
        self.anmi.reset(name);
    }
    pub fn right(&self) -> i32 {
        self.x as i32 + (self.width / 2) as i32
    }
    pub fn left(&self) -> i32{
        self.x as i32 - (self.width / 2) as i32
    }
    pub fn top(&self)->i32{
        self.y as i32 - (self.height / 2) as i32
    }

    pub fn bottom(&self)->i32{
        self.y as i32 + (self.height / 2) as i32
    }

    pub fn rect(&self) -> Rect {
        Rect{x:self.left(),y:self.top(),w:self.width, h:self.height}
    }
    pub fn intersect_with(&self, other:&Sprite) -> Option<Rect>{
        self.rect().intersect(&other.rect())
    }

    // pub fn to_local(&self, rect:&mut Rect){
    //     rect.x -= self.left();
    //     rect.y -= self.top();
    // }

    pub fn pixel_overlap(&self, rhs:&Sprite, intersection:&Rect, frame_mgr:&FrameMgr) -> bool {
        let clip1 = Rect{x:intersection.x - self.left(), y:intersection.y-self.top(), w:intersection.w, h:intersection.h};
        let clip2 = Rect{x:intersection.x - rhs.left(), y:intersection.y-rhs.top(), w:intersection.w, h:intersection.h};
        let img1 = self.current_image(frame_mgr).unwrap_or(&self.image);
        let img2 = rhs.current_image(frame_mgr).unwrap_or(&rhs.image);
        //self.image.test_overlap(&rhs.image, &clip1, &clip2)
        img1.test_overlap(img2, &clip1, &clip2)
    }
    pub fn front_of2(z11:i32, z12:i32, z21:i32, z22:i32) -> bool{
        z11 > z21 || ((z11 == z21) && z12 > z22)
    }
    
    pub fn front_of(&self, other:&Sprite) -> bool {
        self.z > other.z || ((self.z == other.z) && self.z2 > other.z2)
    }
    pub fn update(&mut self, dt: &time::Duration, frame_mgr:&FrameMgr) {
        self.anmi.time_go(dt);
        if let Some(current_img) = self.current_image(frame_mgr){
            self.width = current_img.width();
            self.height = current_img.height();
        }

        let s = dt.as_secs_f32();
        if self.ax != 0.0 {
            self.vx += self.ax * s;
        }
        if self.ay != 0.0 {
            self.vy += self.ay * s;
        }                
        if self.vx != 0.0 || self.ax != 0.0 || self.fx != 0.0 {
            self.x += self.vx * s;
        }
        if self.vy != 0.0 || self.ay != 0.0 || self.fy != 0.0 {
            //println!("{:?}:{:?}:{}", time::Instant::now(), dt, self.vy * s);
            self.y += self.vy * s;
        }
    }
}

pub mod sprite_kind {
    use std::cell::RefCell;

thread_local!{
    static LAST_KIND :RefCell<usize> = RefCell::new(10usize);
}
pub fn create() -> usize {
    LAST_KIND.with(|vp| -> usize {
        let v = vp.take() + 1;
        *vp.borrow_mut() = v;
        v
    })
}

pub fn player() -> usize {
    1usize
}
pub fn projectile()->usize{
    2usize
}
}
