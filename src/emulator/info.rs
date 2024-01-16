use embedded_graphics::Drawable;
use embedded_graphics::primitives::Primitive;
use embedded_graphics::pixelcolor::IntoStorage;

use super::{game::{BMP_HEIGHT, BMP_WIDTH}, resource::{Bitmap, COLORS, Canvas, Rect}};

enum What{
    Score,
    Life,
    CountDown,
}

const SCORE_Y : i32 = 2;
const LIFE_Y : i32= 20;
const CD_Y : i32= 40;

pub struct Info<'a> {
    pixels:[u32; BMP_WIDTH as usize * BMP_HEIGHT as usize],
    show_score:bool,
    score:f32,
    score_bmp:Bitmap,
    score_text_style:embedded_graphics::mono_font::MonoTextStyle<'a,embedded_graphics::pixelcolor::Rgb888>,
    score_rect_style:embedded_graphics::primitives::PrimitiveStyle<embedded_graphics::pixelcolor::Rgb888>,
    drawing:What,
}

impl<'a> embedded_graphics::draw_target::DrawTarget for Info<'a>  {
    //embedded_graphics not allow to implement customer pixelColor
    type Color=embedded_graphics::pixelcolor::Rgb888;

    type Error=&'static str;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = embedded_graphics::Pixel<Self::Color>> {
        for pixel in pixels {
            self.pixels[(pixel.0.y * BMP_WIDTH as i32 + pixel.0.x) as usize] = pixel.1.into_storage();
        }
        Ok(())
    }    
}

impl<'a> embedded_graphics::prelude::Dimensions for Info<'a>{
    fn bounding_box(&self) -> embedded_graphics::primitives::Rectangle {
        embedded_graphics::primitives::Rectangle::new(embedded_graphics::prelude::Point::zero(), embedded_graphics::prelude::Size::new(BMP_WIDTH, BMP_HEIGHT))
    }
}

impl<'a> Info<'a> {    
    pub fn new() -> Info<'a, > {
        let score_text_style = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
                    .font(&embedded_graphics::mono_font::ascii::FONT_4X6)
                    .text_color(Self::from_u32(COLORS[3]))
                    .background_color(Self::from_u32(COLORS[0]))
                    .build();
        let score_rect_style = embedded_graphics::primitives::PrimitiveStyleBuilder::new()
            .stroke_color(score_text_style.text_color.unwrap())
            .stroke_width(1)
            //.fill_color(score_text_style.background_color.unwrap())
            .build();        
        Info{show_score:false,score:0.0,pixels:[0u32;BMP_WIDTH as usize * BMP_HEIGHT as usize],score_bmp:Bitmap::new(0,0), score_text_style, score_rect_style, drawing:What::Score}
    }

    pub fn draw(&self, canvas: &mut Canvas){
        if self.show_score {
            canvas.draw((BMP_WIDTH as usize - self.score_bmp.width()) as i32, 0, &self.score_bmp);
        }
    }

    pub fn change_score(&mut self, del:f32) {
        self.show_score = true;
        self.set_score(self.score + del);
    }
    pub fn set_score(&mut self, n:f32) {
        self.show_score = true;
        self.drawing = What::Score;
        self.score = ((self.score + n) as i32) as f32;
        let next_point = embedded_graphics::text::Text::with_baseline(
            format!("{}", self.score).as_str(), 
            embedded_graphics::prelude::Point::new(2, SCORE_Y + 2), 
            self.score_text_style,
            embedded_graphics::text::Baseline::Top)
            .draw(self).unwrap();
        let rect = embedded_graphics::primitives::Rectangle::new(embedded_graphics::prelude::Point::new(0, SCORE_Y), embedded_graphics::prelude::Size::new((next_point.x + 1) as u32, 10));
            rect.into_styled(self.score_rect_style).draw(self).unwrap();        
        self.score_bmp = Bitmap::new(rect.size.width as usize, rect.size.height as usize);
        let clip = Clip{pixels:&self.pixels, rect:Rect::new(rect.top_left.x, rect.top_left.y, rect.size.width as usize, rect.size.height as usize), idx:0};        
        //TODO:line by line should have better performance
        self.score_bmp.set_data(rect.size.width as usize, clip);
    }
    pub fn from_u32(c:u32) -> embedded_graphics::pixelcolor::Rgb888{
        embedded_graphics::pixelcolor::Rgb888::from(embedded_graphics::pixelcolor::raw::RawU24::new(c))
    }
}

struct Clip<'a>{
    pixels:&'a[u32; BMP_WIDTH as usize * BMP_HEIGHT as usize],
    rect:Rect,
    idx:i32,
}

impl<'a> Iterator for Clip<'a>{
    type Item=u32;

    fn next(&mut self) -> Option<Self::Item> {        
        let y_offset = self.idx / self.rect.w as i32;
        if y_offset >= self.rect.h as i32{
            return None;
        }
        let x_offset = self.idx - y_offset * self.rect.w as i32;
        self.idx += 1;        
        Some(self.pixels[((y_offset + self.rect.y) * BMP_WIDTH as i32 + x_offset + self.rect.x) as usize])
    }
}