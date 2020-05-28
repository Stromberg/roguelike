use rand::{thread_rng, Rng};

use crate::{
    ai::Ai,
    fighter::{DeathCallback, Fighter},
    get_names_under_mouse, inventory_menu,
    item::{cast_confuse, cast_heal, cast_lightning, Item, UseResult},
    map::Map,
    mapbuilder::MapBuilder,
    menu,
    messages::Messages,
    msgbox, mut_two,
    object::Object,
    render_bar, save_game,
    tcoder::{
        Tcod, BAR_WIDTH, CHARACTER_SCREEN_WIDTH, LEVEL_SCREEN_WIDTH, MAP_HEIGHT, MAP_WIDTH,
        MSG_HEIGHT, MSG_WIDTH, MSG_X, PANEL_HEIGHT, PANEL_Y, SCREEN_WIDTH,
    },
};
use colors::{BLACK, DARKER_RED, GREEN, LIGHT_GREY, LIGHT_RED, RED, VIOLET, WHITE, YELLOW};
use input::Event;
use serde::{Deserialize, Serialize};
use tcod::{
    colors,
    console::blit,
    input::{self, Key},
    map::FovAlgorithm,
    BackgroundFlag, Color, Console, TextAlignment,
};

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

pub const PLAYER: usize = 0;

const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 2;

const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic; // default FOV algorithm
const FOV_LIGHT_WALLS: bool = true; // light walls or not
const TORCH_RADIUS: i32 = 10;

// experience and level-ups
const LEVEL_UP_BASE: i32 = 200;
const LEVEL_UP_FACTOR: i32 = 150;

const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};

const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

#[derive(Clone, Copy, Debug, PartialEq)]
enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    map: Map,
    pub messages: Messages,
    pub inventory: Vec<Object>,
    dungeon_level: u32,
    pub objects: Vec<Object>,
    map_builder: MapBuilder,
}

impl Game {
    pub fn new(tcod: &mut Tcod) -> Game {
        // create object representing the player
        let mut player = Object::new(0, 0, '@', "player", WHITE, true);
        player.alive = true;
        player.fighter = Some(Fighter {
            max_hp: 30,
            hp: 30,
            defense: 2,
            power: 5,
            xp: 0,
            on_death: DeathCallback::Player, // <1>
        });

        let mut game = Game {
            // generate map (at this point it's not drawn to the screen)
            map: vec![],
            messages: Messages::new(),
            inventory: vec![], // <1>
            dungeon_level: 1,
            objects: vec![player],
            map_builder: MapBuilder {
                max_rooms: MAX_ROOMS,
                room_min_size: ROOM_MIN_SIZE,
                room_max_size: ROOM_MAX_SIZE,
                max_room_monsters: MAX_ROOM_MONSTERS,
                max_room_items: MAX_ROOM_ITEMS,
            },
        };

        game.initialize_map();
        game.initialise_fov(tcod);

        // a warm welcoming message!
        game.messages.add(
            "Welcome stranger! Prepare to perish in the Tombs of the Ancient Kings.",
            RED,
        );

        game
    }

    fn initialize_map(&mut self) {
        self.map = self.map_builder.build(&mut self.objects);
    }

    pub fn play(&mut self, tcod: &mut Tcod) {
        self.initialise_fov(tcod);

        // force FOV "recompute" first time through the game loop
        let mut previous_player_position = (-1, -1);

        while !tcod.root.window_closed() {
            // clear the screen of the previous frame
            tcod.con.clear();

            match input::check_for_event(input::MOUSE | input::KEY_PRESS) {
                Some((_, Event::Mouse(m))) => tcod.mouse = m,
                Some((_, Event::Key(k))) => tcod.key = k,
                _ => tcod.key = Default::default(),
            }

            // render the screen
            let fov_recompute = previous_player_position != (self.objects[PLAYER].pos()); // <1>
            self.render_all(tcod, fov_recompute);

            tcod.root.flush();

            // level up if needed
            self.level_up(tcod);

            // handle keys and exit game if needed
            previous_player_position = self.objects[PLAYER].pos();
            let player_action = self.handle_keys(tcod);
            if player_action == PlayerAction::Exit {
                save_game(self).unwrap();
                break;
            }

            // let monsters take their turn
            if self.objects[PLAYER].alive && player_action != PlayerAction::DidntTakeTurn {
                for id in 0..self.objects.len() {
                    if self.objects[id].ai.is_some() {
                        self.ai_take_turn(id, tcod);
                    }
                }
            }
        }
    }

    /// return the position of a tile left-clicked in player's FOV (optionally in a
    /// range), or (None,None) if right-clicked.
    pub fn target_tile(&mut self, tcod: &mut Tcod, max_range: Option<f32>) -> Option<(i32, i32)> {
        use tcod::input::KeyCode::Escape;
        loop {
            // render the screen. this erases the inventory and shows the names of
            // objects under the mouse.
            tcod.root.flush();
            let event = input::check_for_event(input::KEY_PRESS | input::MOUSE).map(|e| e.1);
            match event {
                Some(Event::Mouse(m)) => tcod.mouse = m,
                Some(Event::Key(k)) => tcod.key = k,
                None => tcod.key = Default::default(),
            }
            self.render_all(tcod, false);

            let (x, y) = (tcod.mouse.cx as i32, tcod.mouse.cy as i32);

            // accept the target if the player clicked in FOV, and in case a range
            // is specified, if it's in that range
            let in_fov = (x < MAP_WIDTH) && (y < MAP_HEIGHT) && tcod.fov.is_in_fov(x, y);
            let in_range =
                max_range.map_or(true, |range| self.objects[PLAYER].distance(x, y) <= range);
            if tcod.mouse.lbutton_pressed && in_fov && in_range {
                return Some((x, y));
            }

            if tcod.mouse.rbutton_pressed || tcod.key.code == Escape {
                return None; // cancel if the player right-clicked or pressed Escape
            }
        }
    }

    fn handle_keys(&mut self, tcod: &mut Tcod) -> PlayerAction {
        use tcod::input::KeyCode::*;
        use PlayerAction::*;

        let player_alive = self.objects[PLAYER].alive;
        match (tcod.key, tcod.key.text(), player_alive) {
            (
                Key {
                    code: Enter,
                    alt: true,
                    ..
                },
                _,
                _,
            ) => {
                // Alt+Enter: toggle fullscreen
                let fullscreen = tcod.root.is_fullscreen();
                tcod.root.set_fullscreen(!fullscreen);
                DidntTakeTurn
            }
            (Key { code: Escape, .. }, _, _) => return Exit, // exit game
            // movement keys
            (Key { code: Up, .. }, _, true) => {
                self.player_move_or_attack(0, -1);
                TookTurn
            }
            (Key { code: Down, .. }, _, true) => {
                self.player_move_or_attack(0, 1);
                TookTurn
            }
            (Key { code: Left, .. }, _, true) => {
                self.player_move_or_attack(-1, 0);
                TookTurn
            }
            (Key { code: Right, .. }, _, true) => {
                self.player_move_or_attack(1, 0);
                TookTurn
            }
            (Key { code: Text, .. }, "g", true) => {
                // pick up an item
                let item_id = self.objects.iter().position(|object| {
                    object.pos() == self.objects[PLAYER].pos() && object.item.is_some()
                });
                if let Some(item_id) = item_id {
                    self.pick_item_up(item_id);
                }
                DidntTakeTurn
            }
            (Key { code: Text, .. }, "i", true) => {
                // show the inventory
                let inventory_index = inventory_menu(
                    &self.inventory,
                    "Press the key next to an item to use it, or any other to cancel.\n",
                    &mut tcod.root,
                );
                if let Some(inventory_index) = inventory_index {
                    self.use_item(inventory_index, tcod);
                }
                DidntTakeTurn
            }
            (Key { code: Text, .. }, "d", true) => {
                // show the inventory; if an item is selected, drop it
                let inventory_index = inventory_menu(
                    &self.inventory,
                    "Press the key next to an item to drop it, or any other to cancel.\n'",
                    &mut tcod.root,
                );
                if let Some(inventory_index) = inventory_index {
                    self.drop_item(inventory_index);
                }
                DidntTakeTurn
            }
            (Key { code: Text, .. }, "v", true) => {
                // go down stairs, if the player is on them
                let player_on_stairs = self.objects.iter().any(|object| {
                    object.pos() == self.objects[PLAYER].pos() && object.name == "stairs"
                });
                if player_on_stairs {
                    self.next_level(tcod);
                }
                DidntTakeTurn
            }
            (Key { code: Text, .. }, "c", true) => {
                // show character information
                let player = &self.objects[PLAYER];
                let level = player.level;
                let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
                if let Some(fighter) = player.fighter.as_ref() {
                    let msg = format!(
                        "Character information
            
            Level: {}
            Experience: {}
            Experience to level up: {}
            
            Maximum HP: {}
            Attack: {}
            Defense: {}",
                        level,
                        fighter.xp,
                        level_up_xp,
                        fighter.max_hp,
                        fighter.power,
                        fighter.defense
                    );
                    msgbox(&msg, CHARACTER_SCREEN_WIDTH, &mut tcod.root);
                }

                DidntTakeTurn
            }
            _ => DidntTakeTurn,
        }
    }

    fn player_move_or_attack(&mut self, dx: i32, dy: i32) {
        // the coordinates the player is moving to/attacking
        let x = self.objects[PLAYER].x + dx;
        let y = self.objects[PLAYER].y + dy;

        // try to find an attackable object there
        let target_id = self
            .objects
            .iter()
            .position(|object| object.fighter.is_some() && object.pos() == (x, y));

        // attack if target found, move otherwise
        match target_id {
            Some(target_id) => {
                let (player, target) = mut_two(PLAYER, target_id, &mut self.objects);
                player.attack(target, &mut self.messages);
            }
            None => {
                self.move_by(PLAYER, dx, dy);
            }
        }
    }

    /// move by the given amount, if the destination is not blocked
    fn move_by(&mut self, id: usize, dx: i32, dy: i32) {
        let (x, y) = self.objects[id].pos();
        if !is_blocked(x + dx, y + dy, &mut self.map, &mut self.objects) {
            self.objects[id].set_pos(x + dx, y + dy);
        }
    }

    fn move_towards(&mut self, id: usize, target_x: i32, target_y: i32) {
        // vector from this object to the target, and distance
        let dx = target_x - self.objects[id].x;
        let dy = target_y - self.objects[id].y;
        let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

        // normalize it to length 1 (preserving direction), then round it and
        // convert to integer so the movement is restricted to the map grid
        let dx = (dx as f32 / distance).round() as i32;
        let dy = (dy as f32 / distance).round() as i32;
        self.move_by(id, dx, dy);
    }

    /// add to the player's inventory and remove from the map
    fn pick_item_up(&mut self, object_id: usize) {
        if self.inventory.len() >= 26 {
            self.messages.add(
                format!(
                    "Your inventory is full, cannot pick up {}.",
                    self.objects[object_id].name
                ),
                RED,
            );
        } else {
            let item = self.objects.swap_remove(object_id);
            self.messages
                .add(format!("You picked up a {}!", item.name), GREEN);
            self.inventory.push(item);
        }
    }

    /// Advance to the next level
    fn next_level(&mut self, tcod: &mut Tcod) {
        self.messages.add(
            "You take a moment to rest, and recover your strength.",
            VIOLET,
        );
        let heal_hp = self.objects[PLAYER].fighter.map_or(0, |f| f.max_hp / 2);
        self.objects[PLAYER].heal(heal_hp);

        self.messages.add(
            "After a rare moment of peace, you descend deeper into \
         the heart of the dungeon...",
            RED,
        );
        self.dungeon_level += 1;
        self.initialize_map();
        self.initialise_fov(tcod);
    }

    fn ai_take_turn(&mut self, monster_id: usize, tcod: &Tcod) {
        if let Some(ai) = self.objects[monster_id].ai.take() {
            let new_ai = match ai {
                Ai::Basic => self.ai_basic(monster_id, tcod),
                Ai::Confused {
                    previous_ai,
                    num_turns,
                } => self.ai_confused(monster_id, tcod, previous_ai, num_turns),
            };
            self.objects[monster_id].ai = Some(new_ai);
        }
    }

    fn ai_basic(&mut self, monster_id: usize, tcod: &Tcod) -> Ai {
        // a basic monster takes its turn. If you can see it, it can see you
        let (monster_x, monster_y) = self.objects[monster_id].pos();
        if tcod.fov.is_in_fov(monster_x, monster_y) {
            if self.objects[monster_id].distance_to(&self.objects[PLAYER]) >= 2.0 {
                // move towards player if far away
                let (player_x, player_y) = self.objects[PLAYER].pos();
                self.move_towards(monster_id, player_x, player_y);
            } else if self.objects[PLAYER].fighter.map_or(false, |f| f.hp > 0) {
                // close enough, attack! (if the player is still alive.)
                let (monster, player) = mut_two(monster_id, PLAYER, &mut self.objects);
                monster.attack(player, &mut self.messages);
            }
        }
        Ai::Basic
    }

    fn ai_confused(
        &mut self,
        monster_id: usize,
        _tcod: &Tcod,
        previous_ai: Box<Ai>,
        num_turns: i32,
    ) -> Ai {
        if num_turns >= 0 {
            // still confused ...
            // move in a random direction, and decrease the number of turns confused
            self.move_by(
                monster_id,
                thread_rng().gen_range(-1, 2),
                thread_rng().gen_range(-1, 2),
            );
            Ai::Confused {
                previous_ai: previous_ai,
                num_turns: num_turns - 1,
            }
        } else {
            // restore the previous AI (this one will be deleted)
            self.messages.add(
                format!(
                    "The {} is no longer confused!",
                    self.objects[monster_id].name
                ),
                RED,
            );
            *previous_ai
        }
    }

    fn render_all(&mut self, tcod: &mut Tcod, fov_recompute: bool) {
        if fov_recompute {
            // recompute FOV if needed (the player moved or something)
            let player = &self.objects[PLAYER];
            tcod.fov
                .compute_fov(player.x, player.y, TORCH_RADIUS, FOV_LIGHT_WALLS, FOV_ALGO);
        }

        // draw all objects in the list
        let mut to_draw: Vec<_> = self
            .objects
            .iter()
            .filter(|o| {
                tcod.fov.is_in_fov(o.x, o.y)
                    || (o.always_visible && self.map[o.x as usize][o.y as usize].explored)
            })
            .collect();
        to_draw.sort_by(|o1, o2| o1.blocks.cmp(&o2.blocks));
        for object in to_draw {
            object.draw(&mut tcod.con);
        }

        // go through all tiles, and set their background color
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let visible = tcod.fov.is_in_fov(x, y);
                let wall = self.map[x as usize][y as usize].block_sight;
                let color = match (visible, wall) {
                    // outside of field of view:
                    (false, true) => COLOR_DARK_WALL,
                    (false, false) => COLOR_DARK_GROUND,
                    // inside fov:
                    (true, true) => COLOR_LIGHT_WALL,
                    (true, false) => COLOR_LIGHT_GROUND,
                };
                let explored = &mut self.map[x as usize][y as usize].explored;
                if visible {
                    // since it's visible, explore it
                    *explored = true;
                }
                if *explored {
                    // show explored tiles only (any visible tile is explored already)
                    tcod.con
                        .set_char_background(x, y, color, BackgroundFlag::Set);
                }
            }
        }

        blit(
            &tcod.con,
            (0, 0),
            (MAP_WIDTH, MAP_HEIGHT),
            &mut tcod.root,
            (0, 0),
            1.0,
            1.0,
        );

        // prepare to render the GUI panel
        tcod.panel.set_default_background(BLACK);
        tcod.panel.clear();

        // show the player's stats
        let hp = self.objects[PLAYER].fighter.map_or(0, |f| f.hp);
        let max_hp = self.objects[PLAYER].fighter.map_or(0, |f| f.max_hp);
        render_bar(
            &mut tcod.panel,
            1,
            1,
            BAR_WIDTH,
            "HP",
            hp,
            max_hp,
            LIGHT_RED,
            DARKER_RED,
        );

        tcod.panel.print_ex(
            1,
            3,
            BackgroundFlag::None,
            TextAlignment::Left,
            format!("Dungeon level: {}", self.dungeon_level),
        );

        // print the game messages, one line at a time
        let mut y = MSG_HEIGHT as i32;
        for &(ref msg, color) in self.messages.iter().rev() {
            let msg_height = tcod.panel.get_height_rect(MSG_X, y, MSG_WIDTH, 0, msg);
            y -= msg_height;
            if y < 0 {
                break;
            }
            tcod.panel.set_default_foreground(color);
            tcod.panel.print_rect(MSG_X, y, MSG_WIDTH, 0, msg);
        }

        // display names of objects under the mouse
        tcod.panel.set_default_foreground(LIGHT_GREY);
        tcod.panel.print_ex(
            1,
            0,
            BackgroundFlag::None,
            TextAlignment::Left,
            get_names_under_mouse(tcod.mouse, &self.objects, &tcod.fov),
        );

        // blit the contents of `panel` to the root console
        blit(
            &tcod.panel,
            (0, 0),
            (SCREEN_WIDTH, PANEL_HEIGHT),
            &mut tcod.root,
            (0, PANEL_Y),
            1.0,
            1.0,
        );
    }

    fn level_up(&mut self, tcod: &mut Tcod) {
        let player = &mut self.objects[PLAYER];
        let level_up_xp = LEVEL_UP_BASE + player.level * LEVEL_UP_FACTOR;
        // see if the player's experience is enough to level-up
        if player.fighter.as_ref().map_or(0, |f| f.xp) >= level_up_xp {
            // it is! level up
            player.level += 1;
            self.messages.add(
                format!(
                    "Your battle skills grow stronger! You reached level {}!",
                    player.level
                ),
                YELLOW,
            );
            let fighter = player.fighter.as_mut().unwrap();
            let mut choice = None;
            while choice.is_none() {
                // keep asking until a choice is made
                choice = menu(
                    "Level up! Choose a stat to raise:\n",
                    &[
                        format!("Constitution (+20 HP, from {})", fighter.max_hp),
                        format!("Strength (+1 attack, from {})", fighter.power),
                        format!("Agility (+1 defense, from {})", fighter.defense),
                    ],
                    LEVEL_SCREEN_WIDTH,
                    &mut tcod.root,
                );
            }
            fighter.xp -= level_up_xp;
            match choice.unwrap() {
                0 => {
                    fighter.max_hp += 20;
                    fighter.hp += 20;
                }
                1 => {
                    fighter.power += 1;
                }
                2 => {
                    fighter.defense += 1;
                }
                _ => unreachable!(),
            }
        }
    }

    fn use_item(&mut self, inventory_id: usize, tcod: &mut Tcod) {
        use Item::*;
        // just call the "use_function" if it is defined
        if let Some(item) = self.inventory[inventory_id].item {
            let on_use = match item {
                Heal => cast_heal,
                Lightning => cast_lightning,
                Confuse => cast_confuse,
            };
            match on_use(inventory_id, tcod, self) {
                UseResult::UsedUp => {
                    // destroy after use, unless it was cancelled for some reason
                    self.inventory.remove(inventory_id);
                }
                UseResult::Cancelled => {
                    self.messages.add("Cancelled", WHITE);
                }
            }
        } else {
            self.messages.add(
                format!("The {} cannot be used.", self.inventory[inventory_id].name),
                WHITE,
            );
        }
    }

    fn drop_item(&mut self, inventory_id: usize) {
        let mut item = self.inventory.remove(inventory_id);
        item.set_pos(self.objects[PLAYER].x, self.objects[PLAYER].y);
        self.messages
            .add(format!("You dropped a {}.", item.name), YELLOW);
        self.objects.push(item);
    }

    fn initialise_fov(&mut self, tcod: &mut Tcod) {
        // create the FOV map, according to the generated map
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                tcod.fov.set(
                    x,
                    y,
                    !self.map[x as usize][y as usize].block_sight,
                    !self.map[x as usize][y as usize].blocked,
                );
            }
        }

        // unexplored areas start black (which is the default background color)
        tcod.con.clear();
    }
}

pub fn is_blocked(x: i32, y: i32, map: &Map, objects: &[Object]) -> bool {
    // first test the map tile
    if map[x as usize][y as usize].blocked {
        return true;
    }
    // now check for any blocking objects
    objects
        .iter()
        .any(|object| object.blocks && object.pos() == (x, y))
}
