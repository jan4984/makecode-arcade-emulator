use std::fmt;

pub const BMP_WIDTH: u32 = 160;
pub const BMP_HEIGHT: u32 = 120;
pub const DISPL_WIDTH: u32 = 640;
pub const DISPL_HEIGHT: u32 = 480;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum KeyCode {
    Up,
    Down,
    Left,
    Right,
    X,
    Y,
    A,
    B,
    None,
}

impl fmt::Display for KeyCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
        // or, alternatively:
        // fmt::Debug::fmt(self, f)
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum KeyEvt {
    Up(KeyCode),
    Down(KeyCode),
    Repeat(KeyCode),
    Exit,
}
pub struct Game {
    //pub controller: controller::Controller<'a>,
    //pub scene: scene::Scene<'a>,
    pub over: bool,
    pub win: bool,
    //pixels: [u8; 4 * BMP_HEIGHT as usize * BMP_WIDTH as usize],
}

impl Game {
    pub fn new() -> Game {
        Game {
            over: false,
            win: true,
        }
    }

    pub fn over(&mut self, _win: bool) {
        //println!("game over, win ? {win}");
        self.over = true;
    }
}

//API: https://github.com/microsoft/pxt-common-packages/tree/master/libs/game
//https://github.com/microsoft/pxt-common-packages/tree/master/libs/game/docs
