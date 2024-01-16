use std::{cmp::{max, min}, collections::{HashMap}, time};

use super::game::{self, BMP_HEIGHT, BMP_WIDTH};

pub static COLORS: [u32; 0x10] = [
    0,//0
    0xffffffu32,//.to_le_bytes(),//1
    0xff2121u32,//.to_le_bytes(),//2
    0xff93c4u32,//.to_le_bytes(),//3
    0xff8135u32,//.to_le_bytes(),//4
    0xfff609u32,//.to_le_bytes(),//5
    0x249ca3u32,//.to_le_bytes(),//6
    0x78dc52u32,//.to_le_bytes(),//7
    0x003fadu32,//.to_le_bytes(),//8
    0x87f2ffu32,//.to_le_bytes(),//9
    0x8e2ec4u32,//.to_le_bytes(),//a
    0xa4839fu32,//.to_le_bytes(),//b
    0x5c406cu32,//.to_le_bytes(),//c
    0xe5cdc4u32,//.to_le_bytes(),//d
    0x91463du32,//.to_le_bytes(),//e
    0x000000FFu32,//.to_le_bytes(),//f
];

pub fn map_color(idx : &u8) -> u32{
    match idx {
        b'a'..=b'f' => COLORS[(idx - b'a' + 10) as usize], //&[255,0,0,255],
        b'0'..=b'9' => COLORS[(idx - b'0') as usize],      //&[255,255,0,255],//
        _ => COLORS[0],                                     //&[255,0,255,255],//
    }
    // println!("{},{},{}", dest[40],dest[41],dest[42]);
    // println!("{},{},{}", dest[43],dest[44],dest[45])
}


pub fn map_color2(idx : u8) -> u32{
    match idx {
        b'a'..=b'f' => COLORS[(idx - b'a' + 10) as usize], //&[255,0,0,255],
        b'0'..=b'9' => COLORS[(idx - b'0') as usize],      //&[255,255,0,255],//
        _ => COLORS[15],                                     //&[255,0,255,255],//
    }
    // println!("{},{},{}", dest[40],dest[41],dest[42]);
    // println!("{},{},{}", dest[43],dest[44],dest[45])
}

pub struct Bitmap{
    pixels: Vec<u32>,
    w: usize,
}

pub static  BITMAP_EMPTY:Bitmap = Bitmap{pixels:vec![],w:0};

impl Bitmap {
    pub fn rect(&self) -> Rect{
        Rect{x:0,y:0,w:self.w,h:self.height()}
    }
    pub fn new_with_color(w:usize,h:usize,color:u8) -> Bitmap{
        Bitmap{pixels:vec![map_color2(color);w*h], w}
    }

    pub fn new_with_bmp(pixels:Vec<u32>,w:usize) -> Self{
        Bitmap{pixels, w}
    }
    pub fn new(w:usize,h:usize) ->Bitmap{
        Bitmap{pixels:vec![0;w*h], w}
    }

    pub fn new_from_string_without_size(data:&str) -> Bitmap{
        let mut pixels:Vec<u32> = vec![];
        let mut width = 0;
        let mut ix = 0;
        for line in data.split("\n") {
            let line_pixels : Vec<u8> = line.bytes().filter(|&x|x!=b'\n' && x!=b'\r' && x!=b' ').map(|x|if x==b'.' { 0u8 } else {x}).collect();
            if line_pixels.len() == 0 {
                continue;
            }            
            if width == 0 {
                width = line_pixels.len();
            }else{
                assert_eq!(line_pixels.len(), width, "wrong width for line {}: {}", ix, line_pixels.len());
            }
            pixels.extend(line_pixels.iter().map(map_color));
            ix += 1;
        }
        Bitmap{pixels, w: width}
    }

    pub fn set_data<T>(&mut self, w:usize,src:T) where T: Iterator<Item=u32>{
        self.w = w;
        self.pixels=src.collect();
        assert!(self.pixels.len() % self.w == 0);
    }

    pub fn new_from_string(data:&str, w:usize, h:usize) -> Bitmap{
        let pixels:Vec<u8> = data.bytes().filter(|&x|x!=b'\n' && x!=b'\r' && x!=b' ').map(|x|if x==b'.' { 0u8 } else {x}).collect();
        assert_eq!(pixels.len(), w * h);
        Bitmap{pixels:pixels.iter().map(map_color).collect(), w}
    }
    pub fn new_from(data:impl Iterator<Item=u8>, w:usize) -> Bitmap{
        let pixels:Vec<u8> = data.collect();
        assert_eq!(pixels.len() % w, 0);
        Bitmap{pixels:pixels.iter().map(map_color).collect(), w}
    }

    pub fn width(&self) ->usize{
        self.w
    }

    pub fn height(&self) -> usize {
        if self.w == 0 {
            return 0;
        }
        self.pixels.len() / self.w
    }

    fn get_pixel(&self, x:usize, y:usize) -> u32{
        //println!("{}, {}", x, y);
        self.pixels[y*self.w+x]
    }

    pub fn test_overlap(&self, rhs: &Bitmap, clip1: &Rect, clip2: &Rect) -> bool {
        assert_eq!(clip1.w, clip2.w);
        assert_eq!(clip1.h, clip2.h);
        let mut ix;
        let mut iy = 0i32;
        for y in clip1.y..clip1.bottom() {            
            ix = 0;
            for x in clip1.x..clip1.right() {
                //println!("({},{}) <> ({},{})", x, y, clip2.x + ix, clip2.y + iy);
                if self.get_pixel(x as usize, y as usize) != 0 &&  rhs.get_pixel((clip2.x + ix) as usize, (clip2.y + iy) as usize) != 0  {
                    return true;
                }
                ix += 1;
            }
            iy += 1;
        }

        return false;
    }
}

pub fn add_iu(i: i32, u: usize) -> i32 {
    if i.is_negative() {
        u as i32 - i.wrapping_abs() as i32
    } else {
        (u + i as usize) as i32
    }
}

#[derive(PartialEq,Eq,Debug)]
pub struct Rect{
    pub x:i32,
    pub y:i32,
    pub w:usize,
    pub h:usize,
}

pub static CANVAS_RECT: Rect = Rect{x:0,y:0,w:BMP_WIDTH as usize,h:BMP_HEIGHT as usize};

//#[derive(Clone, Copy)]
pub struct Canvas(pub [u32;game::BMP_WIDTH as usize * game::BMP_HEIGHT as usize]);

impl Canvas {
    pub fn new() -> Self {
        Canvas([0;BMP_WIDTH as usize * BMP_HEIGHT as usize])
    }

    // pub fn test_overlap(&self, rhs: &BitmapOnCanvas, clip: &Rect) -> bool {
    //     for x in clip.x..clip.right() {
    //         for y in clip.y..clip.bottom() {
    //             if self.get_pixel(x as usize, y as usize) + rhs.get_pixel(x as usize, y as usize) != 0  {
    //                 return true
    //             }
    //         }
    //     }

    //     false
    // }

    pub fn get_pixel(&self, x:usize, y:usize) -> u32{
        if x >= BMP_WIDTH as usize || y >=BMP_HEIGHT as usize{
            0u32
        }else{
            self.0[y * BMP_WIDTH as usize + x]
        }
    }
    
    pub fn draw(&mut self, x:i32, y:i32, bmp: &Bitmap) -> usize {
        let bmp_width = bmp.w as i32;
        if bmp_width == 0 {
            return 0;
        }
        let bmp_height = (bmp.pixels.len() / bmp.w) as i32;        
        let dest_x = max(0, x);
        let dest_y = max(0, y);
        let dest_bottom = min(120, y + bmp_height);
        if dest_bottom < 0 {
            return 0;
        }
        let dest_right = min(160, x + bmp_width);
        if dest_right < 0 {
            return 0;
        }
        let dest_width = dest_right - dest_x;
        let dest_height = dest_bottom - dest_y;
        
        for i in 0..dest_height{
            let src_start = (dest_y - y + i) * bmp_width + (dest_x - x);
            //self.pixels[(dest_y + i) as usize][dest_x as usize..(dest_x+dest_width) as usize].clone_from_slice(&bmp.pixels[src_start as usize..(src_start+dest_width) as usize]);
            for pixel in src_start..src_start + dest_width {                
                if bmp.pixels[pixel as usize] == 0 {
                    continue;
                }
                let dest_idx:&mut u32 = &mut self.0[(dest_y + i) as usize * BMP_WIDTH as usize + (dest_x as usize + (pixel-src_start) as usize)];
                *dest_idx = bmp.pixels[pixel as usize];
            }
        }
        return (dest_height * dest_width) as usize;
    }
}

// #[cfg(test)]
// mod test_bitmap{
//     use super::{Bitmap, Canvas};

//     #[test]
//     fn test_draw(){
//         let mut canvas = Canvas::new();
//         assert_eq!(1usize, canvas.draw(0,0, &Bitmap::new_from((&[1u8]).into_iter().cloned(), 1)));
//         assert_eq!(canvas.pixels[0][0], 1u32);
//         assert_eq!(canvas.pixels[1..120], [[0u8;160];119]);

//         assert_eq!(4 * 16, canvas.draw(10,15, &Bitmap::new_from((&[5u8;64]).into_iter().cloned(), 16)));
//         for i in 15..15+4{
//             assert_eq!(canvas.pixels[i][10..26], [5u8;16], "line {}", i);
//         }
//         assert_ne!(canvas.pixels[19][10..26], [5u8;16], "line 19");

//         assert_eq!(0usize, canvas.draw(-20, -30, &Bitmap::new_from((&[0xfu8;20*30]).into_iter().cloned(), 20)));
//         assert_eq!(0usize, canvas.draw(-20, -30, &Bitmap::new_from((&[0xfu8;30*40]).into_iter().cloned(), 40)));
//         assert_eq!(20usize * 2, canvas.draw(-20, -30, &Bitmap::new_from((&[0xfu8;32*40]).into_iter().cloned(), 40)));
//         assert_eq!(canvas.pixels[0][0..20], [0xfu8;20]);
//         assert_eq!(canvas.pixels[1][0..20], [0xfu8;20]);
//         assert_ne!(canvas.pixels[2][0..20], [0xfu8;20]);

//         assert_eq!(160 - 5, canvas.draw(5, 119, &Bitmap::new_from((&[0xau8;163*30]).into_iter().cloned(), 163)));
//         assert_eq!(canvas.pixels[119][5..], [0xau8;160-5]);
//     }
// }

impl Default for Rect{
    fn default() -> Rect{
        Rect{x:0,y:0,w:0,h:0}
    }
}

impl Rect{    
    pub fn new(left:i32, top:i32, width:usize, height:usize) -> Rect{
        Rect{x:left, y:top, w:width, h:height}
    }
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let x = max(self.x, other.x);
        let right = min(self.right(), other.right());
        if x >= right {
            return None;
        }
        let y = max(self.y, other.y);
        let bottom = min(self.bottom(), other.bottom());
        if y >= bottom {
            return None;
        }
        Some(Rect{x, y, w:(right - x) as usize, h:(bottom - y) as usize})
    }
    
    pub fn right(&self) -> i32 {
        add_iu(self.x, self.w)
    }
    
    pub fn bottom(&self) -> i32 {
        add_iu(self.y, self.h)
    }
}

#[cfg(test)]
mod tests_rect{
    use crate::emulator::resource::Rect;

    #[test]
    fn right_bottom(){
        let r = Rect{x:-5,y:-10,w:100,h:200};
        assert_eq!(r.right(), 95);        
        assert_eq!(r.bottom(), 190);        
    }

    #[test]
    fn test_intersect(){
        let mut r1 = Rect{x:-5,y:-10,w:100,h:200};
        let r2 = Rect{x: 0, y:0, w: 10, h: 300};        
        assert_eq!(r1.intersect(&r2).unwrap_or_default(), Rect{x:0, y:0, w:10, h:190});
        assert_eq!(r2.intersect(&r1).unwrap_or_default(), Rect{x:0, y:0, w:10, h:190});
        
        r1 = Rect{x:11,y:12,w:5,h:10};
        assert_eq!(r1.intersect(&r2), None);
        assert_eq!(r2.intersect(&r1), None);

        r1 = Rect{x:2,y:300,w:5,h:10};
        assert_eq!(r1.intersect(&r2),None);
        assert_eq!(r2.intersect(&r1),None);

        r1 = Rect{x:2,y:299,w:5,h:10};
        assert_eq!(r1.intersect(&r2).unwrap_or_default(), Rect{x:2,y:299,w:5,h:1});
        assert_eq!(r2.intersect(&r1).unwrap_or_default(), Rect{x:2,y:299,w:5,h:1});
        
        r1 = Rect{x:5,y:12,w:5,h:10};
        assert_eq!(r1.intersect(&r2).unwrap_or_default(),r1);
        assert_eq!(r2.intersect(&r1).unwrap_or_default(),r1);
    }
}


pub struct Frames {
    bmp: Vec<Bitmap>,
    interval: time::Duration,        
}

impl Frames {
    pub fn new(interval :time::Duration) -> Frames {
        Frames{bmp:vec![], interval}
    }
    pub fn add_bmp(&mut self, bmp: Bitmap) {
        self.bmp.push(bmp);
    }
}

pub struct FrameMgr {
    pub frames: HashMap<String, Frames>
}

impl FrameMgr {
    pub fn create(&mut self,name:&str, interval_ms: u32) {        
        self.frames.insert(String::from(name), Frames::new(time::Duration::from_millis(interval_ms as u64)));
        //println!("created animation {}, animations len:{}", name, self.frames.len());
    }

    pub fn append(&mut self, name: &str, bmp : Bitmap) {
        //println!("append frame to {}. animations len:{}", name, self.frames.len());
        self.frames.get_mut(name).unwrap().add_bmp(bmp);
    }
}

#[derive(Clone)]
pub struct Animation{
    pub current_name: String,
    //pub actived: bool,
    pub elapsed: time::Duration,
}

impl Animation {
    pub fn new() -> Animation {
        Animation{
            current_name: "".to_string(),
            elapsed: time::Duration::from_millis(0),
            //actived: false,
        }
    }

    pub fn reset(&mut self, name:String ) {
        if self.current_name == name {
            return;
        }
        self.current_name = name;
        self.elapsed = time::Duration::from_millis(0);
    }

    pub fn time_go(&mut self, dt:&time::Duration) {
        self.elapsed += *dt;
    }

    pub fn current<'a>(&self, fmr:&'a FrameMgr) -> Option<&'a Bitmap> {        
        if self.current_name == "" {
            return None;
        }
         
        //println!("animation name: {}, animation count:{}", self.current_name, fmr.frames.len());
        //println!("len:{}", fmr.frames.get(&self.current_name).unwrap().bmp.len());
        if  fmr.frames.get(&self.current_name).is_none() {
            None
        }else{
            let frames = fmr.frames.get(&self.current_name).unwrap();            
            let mut index = (self.elapsed.as_millis() / frames.interval.as_millis()) as usize;
            index = index % frames.bmp.len();
            Some(&frames.bmp[index])
        }        
    }
}