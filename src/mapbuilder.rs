use crate::{
    game::{is_blocked, PLAYER},
    item::create_item,
    map::{create_h_tunnel, create_room, create_v_tunnel, Map, Tile},
    monsters::create_monster,
    object::Object,
    rect::Rect,
    tcoder::{MAP_HEIGHT, MAP_WIDTH},
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tcod::colors::WHITE;

#[derive(Serialize, Deserialize)]
pub struct MapBuilder {
    pub max_rooms: i32,
    pub room_min_size: i32,
    pub room_max_size: i32,
    pub max_room_monsters: i32,
    pub max_room_items: i32,
}

impl MapBuilder {
    pub fn build(&self, objects: &mut Vec<Object>) -> Map {
        // fill map with "unblocked" tiles
        let mut map = vec![vec![Tile::wall(); MAP_HEIGHT as usize]; MAP_WIDTH as usize];

        // Player is the first element, remove everything else.
        // NOTE: works only when the player is the first object!
        assert_eq!(&objects[PLAYER] as *const _, &objects[0] as *const _);
        objects.truncate(1);

        let mut rooms = vec![];

        for _ in 0..self.max_rooms {
            // random width and height
            let w = rand::thread_rng().gen_range(self.room_min_size, self.room_max_size + 1);
            let h = rand::thread_rng().gen_range(self.room_min_size, self.room_max_size + 1);
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
                self.place_objects(new_room, &mut map, objects);

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

    fn place_objects(&self, room: Rect, map: &mut Map, objects: &mut Vec<Object>) {
        // choose random number of monsters
        let num_monsters = rand::thread_rng().gen_range(0, self.max_room_monsters + 1);

        for _ in 0..num_monsters {
            // choose random spot for this monster
            let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
            let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

            if !is_blocked(x, y, map, objects) {
                objects.push(create_monster(x, y));
            }

            // choose random number of items
            let num_items = rand::thread_rng().gen_range(0, self.max_room_items + 1);

            for _ in 0..num_items {
                // choose random spot for this item
                let x = rand::thread_rng().gen_range(room.x1 + 1, room.x2);
                let y = rand::thread_rng().gen_range(room.y1 + 1, room.y2);

                // only place it if the tile is not blocked
                if !is_blocked(x, y, map, objects) {
                    objects.push(create_item(x, y));
                }
            }
        }
    }
}
