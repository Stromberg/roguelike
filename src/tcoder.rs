use tcod::console::{Offscreen, Root};
use tcod::{
    input::{Key, Mouse},
    map::Map as FovMap,
    FontLayout, FontType,
};

// actual size of the window
pub const SCREEN_WIDTH: i32 = 80;
pub const SCREEN_HEIGHT: i32 = 50;

pub const BAR_WIDTH: i32 = 20;
pub const PANEL_HEIGHT: i32 = 7;
pub const PANEL_Y: i32 = SCREEN_HEIGHT - PANEL_HEIGHT;
pub const INVENTORY_WIDTH: i32 = 50;
pub const LEVEL_SCREEN_WIDTH: i32 = 40;
pub const CHARACTER_SCREEN_WIDTH: i32 = 30;

pub const MSG_X: i32 = BAR_WIDTH + 2;
pub const MSG_WIDTH: i32 = SCREEN_WIDTH - BAR_WIDTH - 2;
pub const MSG_HEIGHT: usize = PANEL_HEIGHT as usize - 1;

// size of the map
pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub struct Tcod {
    pub root: Root,
    pub con: Offscreen,
    pub panel: Offscreen,
    pub fov: FovMap,
    pub key: Key,
    pub mouse: Mouse,
}

impl Tcod {
    pub fn new() -> Tcod {
        let root = Root::initializer()
            .font("arial10x10.png", FontLayout::Tcod)
            .font_type(FontType::Greyscale)
            .size(SCREEN_WIDTH, SCREEN_HEIGHT)
            .title("Rust/libtcod tutorial")
            .init();

        Tcod {
            root,
            con: Offscreen::new(MAP_WIDTH, MAP_HEIGHT),
            panel: Offscreen::new(SCREEN_WIDTH, PANEL_HEIGHT),
            fov: FovMap::new(MAP_WIDTH, MAP_HEIGHT),
            key: Default::default(),
            mouse: Default::default(),
        }
    }
}
