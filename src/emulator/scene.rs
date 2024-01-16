use std::{cell::RefCell, cmp::Ordering, collections::{HashMap, HashSet}, ops::Deref, rc::Rc};
use crate::emulator::{resource::CANVAS_RECT, sprite::Flag};

use super::{effect::{Effect, SceneEffect}, resource::{Bitmap, Canvas, FrameMgr}};
use super::sprite::Sprite;
#[derive(Copy, Clone)]
struct PixelLine<T> {
    points: [T; 160],
}

struct PixelScreen<T> {
    lines: [PixelLine<T>; 120],
}

// impl std::marker::Copy for Vec::<u32> {

// }

const CONST_VEC_U32: Vec<u32> = vec![];
const CONST_LINE: PixelLine<Vec<u32>> = PixelLine {
    points: [CONST_VEC_U32; 160],
};

// #[derive(PartialEq, Eq, Hash)]
// struct SpriteKindPair(SpriteKind,SpriteKind);


pub struct Scene {
    last_idx: usize,
    sprites: HashMap<usize, Sprite>,
    //overlap_callbacks: HashMap<SpriteKindPair, fn(&sprite::Sprite,&sprite::Sprite)>,
    overlap_detections: HashSet<(usize, usize)>,
    //canvas: Canvas,
    bgi: Canvas,
    current_z: u32,
    pub frame_mgr:Rc<RefCell<FrameMgr>>,
    effect:Box<dyn Effect>,
}

impl Scene {
    pub fn new(fmr:Rc<RefCell<FrameMgr>>) -> Scene {
        Scene {
            last_idx: 0usize,
            sprites: HashMap::new(),
            overlap_detections: HashSet::new(),
            //canvas: Canvas::new(),
            bgi: Canvas::new(),
            current_z: 1u32,
            frame_mgr:fmr,
            effect:SceneEffect::dummy(),
        }
    }
    pub fn active_effect(&mut self, effect:Box<dyn Effect>) {
        self.effect = effect;
    }
    pub fn add_sprite(& mut self, mut s: Sprite) -> usize {        
        self.current_z +=1;
        s.z2 = self.current_z as i32;
        self.last_idx += 1;
        self.sprites.insert(self.last_idx, s);        
        println!("sprite {} added", self.last_idx);
        self.last_idx
    }

    pub fn remove_sprite(&mut self, i: usize) {
        self.sprites.remove(&i);
    }

    pub fn get_mut(&mut self, i: usize) -> & mut Sprite {
        self.sprites.get_mut(&i).unwrap()
    }

    pub fn get(&self, i: usize) -> &Sprite {
        //println!("to get sprite at {}", i);
        self.sprites.get(&i).unwrap()
    }

    pub fn set_bgi(&mut self, img: Bitmap){
        self.bgi.draw(0, 0, &img);
    }

    pub fn draw(&self, canvas:&mut Canvas){
        super::profile_method!(draw);

        super::profile_section!(new_canvas);
        canvas.0 = self.bgi.0;
        drop(new_canvas);

        super::profile_section!(draw_background);
        //canvas.draw(0,0, &self.bgi);
        drop(draw_background);

        super::profile_section!(draw_scene_effect);
        self.effect.draw(canvas);
        drop(draw_scene_effect);

        super::profile_section!(draw_children);
        let mut z_ordered:Vec<&Sprite> = self.sprites.values().filter(|sp|sp.flag & Flag::Invisible.u32() == 0).collect();
        z_ordered.sort_by(|&a,&b|if b.front_of(a) { Ordering::Less } else {Ordering::Greater});
        let frames_p = self.frame_mgr.borrow();
        let frames = frames_p.deref();
        for sp in z_ordered {            
            let img = sp.current_image(frames).unwrap_or(sp.native_image());
            canvas.draw(sp.left(), sp.top(), img);
        }
        drop(draw_children);
    }

    pub fn check_overlaps(&self) -> Vec<(usize, usize, usize, usize)>{
        if self.overlap_detections.len() <= 0 {
            return vec![];
        }
        let mut k1s: Vec<usize> = self.overlap_detections.iter().map(|k12| k12.0).collect();
        let mut k2s: Vec<usize> = self.overlap_detections.iter().map(|k12| k12.1).collect();
        k1s.dedup();
        k2s.dedup();
        let filter_by_kind_fn = |kind: usize| {
            self.sprites
                .iter()
                .filter_map(|(i, v)| if v.kind == kind { Some(i) } else { None })
                .collect()
        };
        let frames_p = self.frame_mgr.borrow();
        let frames = frames_p.deref();

        //TODO: multi-thread for performance, or find better solution?
        let mut overlaps: HashSet<(usize, usize, usize, usize)> = HashSet::new();
        for k1 in &k1s {
            let _sp1_indexes: Vec<&usize> = filter_by_kind_fn(*k1);
            for sp1i in _sp1_indexes {
                for k2 in &k2s {
                    let _check1on2 = self.overlap_detections.contains(&(*k1, *k2));
                    let _check2on1 = self.overlap_detections.contains(&(*k2, *k1));
                    if !_check1on2 && !_check2on1 {
                        continue;
                    }

                    let _sp2_indexes: Vec<&usize> = filter_by_kind_fn(*k2);
                    for sp2i in _sp2_indexes {
                        if overlaps.contains(&(*sp1i, *sp2i, *k1, *k2)) || sp1i == sp2i{
                            continue;
                        }

                        let sp1 = &self.get(*sp1i);
                        let sp2 = &self.get(*sp2i);
                        if !match sp1.intersect_with(sp2) {
                            Some(rect) => sp1.pixel_overlap(sp2, &rect, frames),
                            None => false,
                        }{
                            continue;
                        }
                        overlaps.insert((*sp1i, *sp2i, *k1, * k2));
                        overlaps.insert((*sp2i, *sp1i, *k2, *k1));
                    }
                }
            }
        }

        overlaps.into_iter().filter(|k|self.overlap_detections.contains(&(k.2, k.3))).collect()
    }
    pub fn add_overlap_detection(&mut self, kind1: usize, kind2: usize){
        self.overlap_detections.insert((kind1, kind2));
    }

    pub fn update(&mut self, dt: &std::time::Duration) {
        super::profile_fn!(scene_update);

        {
            super::profile_section!(sprite_update);
            let frames_p = self.frame_mgr.borrow();
            let frames = frames_p.deref();
            //update all sprites
            for sp in self.sprites.values_mut(){
                sp.update(dt, frames);
            }
        }
        {
            super::profile_section!(effect_update);
            self.effect.update(dt);
        }

        {
            let mut to_remove:Vec<usize>=vec![];
            for (sp_i, sp) in self.sprites.iter(){
                if (Flag::Autodestory.u32() & sp.flag != 0) && sp.rect().intersect(&CANVAS_RECT).is_none(){
                    to_remove.push(*sp_i as usize);
                }
            }
            for ele in to_remove {
                println!("sprite {ele} auto destoried");
                self.sprites.remove(&ele);
            }
        }
        
        // {            
        //     //paint
        //     self.draw();
        // }
    }    
}

#[cfg(test)]
mod test_overlap {
    use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};

    use crate::emulator::{resource::{Bitmap, FrameMgr}, sprite::{self, sprite_kind}};
    use crate::emulator::resource::Rect;

    #[test]
    fn test_overlap() {
        let fmr = Rc::new(RefCell::new(FrameMgr{frames:HashMap::new()}));
        let frames_p = fmr.borrow();
        let frames = frames_p.deref();
        let img1 = r"
12345678
..1....2
..1....3
..1....4
..1....5
";
        let img2 = r"
..........
.5........
.4........
.3........
.2........
..........
";
        //123456789a
        let mut sp1 = sprite::Sprite::new_with_bmp(
            Bitmap::new_from_string(img1, 8, 5),
            sprite_kind::player(),
        );
        let mut sp2 = sprite::Sprite::new_with_bmp(
            Bitmap::new_from_string(img2, 10, 6),
            sprite_kind::player(),
        );

        let mut rect = Rect::new(0, 0, 8, 5);
        assert_eq!(rect, sp1.intersect_with(&sp2).unwrap());
        assert_eq!(rect, sp2.intersect_with(&sp1).unwrap());
        assert!(!sp1.pixel_overlap(&sp2, &rect, frames));
        assert!(!sp2.pixel_overlap(&sp1, &rect, frames));

        sp1.x += 2.0;
        sp1.y += 1.0;
        rect = Rect::new(2, 1, 8, 5);
        assert_eq!(sp1.intersect_with(&sp2).unwrap(), rect);
        assert_eq!(sp2.intersect_with(&sp1).unwrap(), rect);
        assert!(!sp2.pixel_overlap(&sp1, &rect, frames));
        assert!(!sp2.pixel_overlap(&sp2, &rect, frames));

        sp1.x = 4.0;
        sp1.y = 3.0;
        rect = Rect::new(4, 3, 6, 3);
        assert_eq!(sp1.intersect_with(&sp2).unwrap(), rect);
        assert_eq!(sp2.intersect_with(&sp1).unwrap(), rect);
        assert!(!sp2.pixel_overlap(&sp1, &rect, frames));
        assert!(!sp1.pixel_overlap(&sp2, &rect, frames));

        sp1.x = -1.0;
        sp1.y = -3.0;
        rect = Rect::new(0, 0, 7, 2);
        assert_eq!(sp1.intersect_with(&sp2).unwrap(), rect);
        assert_eq!(sp2.intersect_with(&sp1).unwrap(), rect);
        assert!(sp2.pixel_overlap(&sp1, &rect, frames));
        assert!(sp1.pixel_overlap(&sp2, &rect, frames));

        sp1.x = -1.0;
        sp1.y = -1.0;
        sp2.x = 5.0;
        sp2.y = 2.0;
        rect = Rect::new(5, 2, 2, 2);
        assert_eq!(sp1.intersect_with(&sp2).unwrap(), rect);
        assert_eq!(sp2.intersect_with(&sp1).unwrap(), rect);
        assert!(sp1.pixel_overlap(&sp2, &rect, frames));
        assert!(sp2.pixel_overlap(&sp1, &rect, frames));

        sp1.x = -3.0;
        sp2.x = 5.0;
        assert_eq!(sp1.intersect_with(&sp2), None);
        assert_eq!(sp2.intersect_with(&sp1), None);

        sp1.x = -3.0;
        sp1.y = -5.0;
        sp2.x = -1.0;
        sp2.y = -6.0;
        rect = Rect::new(-1, -5, 6, 5);
        assert_eq!(sp1.intersect_with(&sp2).unwrap(), rect);
        assert_eq!(sp2.intersect_with(&sp1).unwrap(), rect);
        assert!(sp1.pixel_overlap(&sp2, &rect, frames));
        assert!(sp2.pixel_overlap(&sp1, &rect, frames));

        sp1.x = 1.0;
        sp1.y = 0.0;
        sp2.x = 7.0;
        sp2.y = 0.0;
        rect = Rect::new(7, 0, 2, 5);
        assert_eq!(sp1.intersect_with(&sp2).unwrap(), rect);
        assert_eq!(sp2.intersect_with(&sp1).unwrap(), rect);
        assert!(sp1.pixel_overlap(&sp2, &rect, frames));
        assert!(sp2.pixel_overlap(&sp1, &rect, frames));
    }
}

