use crate::{
    ai::Ai,
    fighter::{DeathCallback, Fighter},
    map::{Map, Tile},
    messages::Messages,
    object::Object,
    rect::Rect,
};
use colors::{GREEN, LIGHT_YELLOW, RED, VIOLET, WHITE};
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::cmp;
use tcod::{colors, map::FovAlgorithm, Color};

//parameters for dungeon generator
const ROOM_MAX_SIZE: i32 = 10;
const ROOM_MIN_SIZE: i32 = 6;
const MAX_ROOMS: i32 = 30;

// size of the map
pub const MAP_WIDTH: i32 = 80;
pub const MAP_HEIGHT: i32 = 43;

pub const PLAYER: usize = 0;

const MAX_ROOM_MONSTERS: i32 = 3;
const MAX_ROOM_ITEMS: i32 = 2;

pub const FOV_ALGO: FovAlgorithm = FovAlgorithm::Basic; // default FOV algorithm
pub const FOV_LIGHT_WALLS: bool = true; // light walls or not
pub const TORCH_RADIUS: i32 = 10;

pub const HEAL_AMOUNT: i32 = 4;
pub const LIGHTNING_DAMAGE: i32 = 40;
pub const LIGHTNING_RANGE: i32 = 5;
pub const CONFUSE_RANGE: i32 = 8;
pub const CONFUSE_NUM_TURNS: i32 = 10;

// experience and level-ups
pub const LEVEL_UP_BASE: i32 = 200;
pub const LEVEL_UP_FACTOR: i32 = 150;

pub const COLOR_DARK_WALL: Color = Color { r: 0, g: 0, b: 100 };
pub const COLOR_LIGHT_WALL: Color = Color {
    r: 130,
    g: 110,
    b: 50,
};

pub const COLOR_DARK_GROUND: Color = Color {
    r: 50,
    g: 50,
    b: 150,
};
pub const COLOR_LIGHT_GROUND: Color = Color {
    r: 200,
    g: 180,
    b: 50,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerAction {
    TookTurn,
    DidntTakeTurn,
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub map: Map,
    pub messages: Messages,
    pub inventory: Vec<Object>,
    pub dungeon_level: u32,
}

pub fn make_map(objects: &mut Vec<Object>) -> Map {
    // fill map with "unblocked" tiles
    let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

    // Player is the first element, remove everything else.
    // NOTE: works only when the player is the first object!
    assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
    objects.truncate(1);

    let mut rooms = vec![];

    for _ in 0..MAX_ROOMS {
        // random width and height
        let w = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        let h = rand::thread_rng().gen_range(ROOM_MIN_SIZE, ROOM_MAX_SIZE + 1);
        // random position without going out of the boundaries of the map
        let x = rand::thread_rng().gen_range(0, MAP_WIDTH - w);
        let y = rand::thread_rng().gen_range(0, MAP_HEIGHT - h);

        let new_room = Rect::new(x, y, w, h);

        // run through the other rooms and see if they intersect with this one
        let failed = rooms
            .iter()
            .any(|other_room| new_room.intersects_with(other_room));

        if !failed {
            // this means there are no intersections, so this room is valid

            // "paint" it to the map's tiles
            create_room(new_room, &mut map);
            place_objects(new_room, &map, objects);

            // center coordinates of the new room, will be useful later
            let (new_x, new_y) = new_room.center();

            if rooms.is_empty() {
                // this is the first room, where the player starts at
                objects[PLAYER].x = new_x;
                objects[PLAYER].y = new_y;
            } else {
                // all rooms after the first:
                // connect it to the previous room with a tunnel

                // center coordinates of the previous room
                let (prev_x, prev_y) = rooms[rooms.len() - 1].center();

                // toss a coin (random bool value -- either true or false)
                if rand::random() {
                    // first move horizontally, then vertically
                    create_h_tunnel(prev_x, new_x, prev_y, &mut map);
                    create_v_tunnel(prev_y, new_y, new_x, &mut map);
                } else {
                    // first move vertically, then horizontally
                    create_v_tunnel(prev_y, new_y, prev_x, &mut map);
                    create_h_tunnel(prev_x, new_x, new_y, &mut map);
                }
            }

            // finally, append the new room to the list
            rooms.push(new_room);
        }
    }

    // create stairs at the center of the last room
    let (last_room_x, last_room_y) = rooms[rooms.len() - 1].center();
    let mut stairs = Object::new(last_room_x, last_room_y, '<', "stairs", WHITE, false);
    stairs.always_visible = true;
    objects.push(stairs);

    map
}

pub fn create_room(room: Rect, map: &mut Map) {
    // go through the tiles in the rectangle and make them passable
    for x in (room.x1 + 1)..room.x2 {
        for y in (room.y1 + 1)..room.y2 {
            map[x as usize][y as usize] = Tile::empty();
        }
    }
}

pub fn create_h_tunnel(x1: i32, x2: i32, y: i32, map: &mut Map) {
    // horizontal tunnel. `min()` and `max()` are used in case `x1 > x2`
    for x in cmp::min(x1, x2)..(cmp::max(x1, x2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

pub fn create_v_tunnel(y1: i32, y2: i32, x: i32, map: &mut Map) {
    // vertical tunnel
    for y in cmp::min(y1, y2)..(cmp::max(y1, y2) + 1) {
        map[x as usize][y as usize] = Tile::empty();
    }
}

pub fn place_objects(room: Rect, map: &Map, objects: &mut Vec<Object>) {
    // choose random number of monsters
    let num_monsters = rand::thread_rng().gen_range(0, MAX_ROOM_MONSTERS + 1);

    for _ in 0..num_monsters {
        // choose random spot for this monster
        let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
        let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

        if !is_blocked(x, y, map, objects) {
            // monster random table
            let monster_chances = &mut [
                Weighted {
                    weight: 80,
                    item: "orc",
                },
                Weighted {
                    weight: 20,
                    item: "troll",
                },
            ];
            let monster_choice = WeightedChoice::new(monster_chances);

            let mut monster = match monster_choice.ind_sample(&mut rand::thread_rng()) {
                "orc" => {
                    // create an orc
                    let mut orc = Object::new(x, y, 'o', "orc", colors::DESATURATED_GREEN, true);
                    orc.fighter = Some(Fighter {
                        max_hp: 10,
                        hp: 10,
                        defense: 0,
                        power: 3,
                        xp: 35,
                        on_death: DeathCallback::Monster,
                    });
                    orc.ai = Some(Ai::Basic);
                    orc
                }
                "troll" => {
                    let mut troll = Object::new(x, y, 'T', "troll", colors::DARKER_GREEN, true);
                    troll.fighter = Some(Fighter {
                        max_hp: 16,
                        hp: 16,
                        defense: 1,
                        power: 4,
                        xp: 100,
                        on_death: DeathCallback::Monster,
                    });
                    troll.ai = Some(Ai::Basic);
                    troll
                }
                _ => unreachable!(),
            };

            monster.alive = true;
            objects.push(monster);
        }

        // choose random number of items
        let num_items = rand::thread_rng().gen_range(0, MAX_ROOM_ITEMS + 1);

        for _ in 0..num_items {
            // choose random spot for this item
            let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
            let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

            // only place it if the tile is not blocked
            if !is_blocked(x, y, map, objects) {
                // item random table
                let item_chances = &mut [
                    Weighted {
                        weight: 70,
                        item: Item::Heal,
                    },
                    Weighted {
                        weight: 10,
                        item: Item::Lightning,
                    },
                    Weighted {
                        weight: 10,
                        item: Item::Confuse,
                    },
                ];
                let item_choice = WeightedChoice::new(item_chances);

                let mut item = match item_choice.ind_sample(&mut rand::thread_rng()) {
                    Item::Heal => {
                        // create a healing potion
                        let mut object = Object::new(x, y, '!', "healing potion", VIOLET, false);
                        object.item = Some(Item::Heal);
                        object
                    }
                    Item::Lightning => {
                        // create a lightning bolt scroll
                        let mut object =
                            Object::new(x, y, '#', "scroll of lightning bolt", LIGHT_YELLOW, false);
                        object.item = Some(Item::Lightning);
                        object
                    }
                    Item::Confuse => {
                        // create a confuse scroll
                        let mut object =
                            Object::new(x, y, '#', "scroll of confusion", LIGHT_YELLOW, false);
                        object.item = Some(Item::Confuse);
                        object
                    }
                };

                item.always_visible = true;
                objects.push(item);
            }
        }
    }
}

pub fn move_towards(id: usize, target_x: i32, target_y: i32, map: &Map, objects: &mut [Object]) {
    // vector from this object to the target, and distance
    let dx = target_x - objects[id].x;
    let dy = target_y - objects[id].y;
    let distance = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();

    // normalize it to length 1 (preserving direction), then round it and
    // convert to integer so the movement is restricted to the map grid
    let dx = (dx as f32 / distance).round() as i32;
    let dy = (dy as f32 / distance).round() as i32;
    move_by(id, dx, dy, map, objects);
}

/// move by the given amount, if the destination is not blocked
pub fn move_by(id: usize, dx: i32, dy: i32, map: &Map, objects: &mut [Object]) {
    let (x, y) = objects[id].pos();
    if !is_blocked(x + dx, y + dy, map, objects) {
        objects[id].set_pos(x + dx, y + dy);
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

/// add to the player's inventory and remove from the map
pub fn pick_item_up(object_id: usize, game: &mut Game, objects: &mut Vec<Object>) {
    if game.inventory.len() >= 26 {
        game.messages.add(
            format!(
                "Your inventory is full, cannot pick up {}.",
                objects[object_id].name
            ),
            RED,
        );
    } else {
        let item = objects.swap_remove(object_id);
        game.messages
            .add(format!("You picked up a {}!", item.name), GREEN);
        game.inventory.push(item);
    }
}
