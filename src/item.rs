use crate::{
    ai::Ai,
    game::{Game, PLAYER},
    object::Object,
    tcoder::Tcod,
};
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
use serde::{Deserialize, Serialize};
use tcod::colors::{LIGHT_BLUE, LIGHT_CYAN, LIGHT_GREEN, LIGHT_VIOLET, LIGHT_YELLOW, RED, VIOLET};

const HEAL_AMOUNT: i32 = 4;
const LIGHTNING_DAMAGE: i32 = 40;
const LIGHTNING_RANGE: i32 = 5;
const CONFUSE_RANGE: i32 = 8;
const CONFUSE_NUM_TURNS: i32 = 10;

pub enum UseResult {
    UsedUp,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Heal,
    Lightning,
    Confuse,
}

pub fn create_item(x: i32, y: i32) -> Object {
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
            let mut object = Object::new(x, y, '#', "scroll of confusion", LIGHT_YELLOW, false);
            object.item = Some(Item::Confuse);
            object
        }
    };

    item.always_visible = true;
    item
}

pub fn cast_heal(_inventory_id: usize, _tcod: &mut Tcod, game: &mut Game) -> UseResult {
    // heal the player
    if let Some(fighter) = game.objects[PLAYER].fighter {
        if fighter.hp == fighter.max_hp {
            game.messages.add("You are already at full health.", RED);
            return UseResult::Cancelled;
        }
        game.messages
            .add("Your wounds start to feel better!", LIGHT_VIOLET);
        game.objects[PLAYER].heal(HEAL_AMOUNT);
        return UseResult::UsedUp;
    }
    UseResult::Cancelled
}

pub fn cast_lightning(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game) -> UseResult {
    // find closest enemy (inside a maximum range and damage it)
    let monster_id = closest_monster(tcod, &game.objects, LIGHTNING_RANGE);
    if let Some(monster_id) = monster_id {
        // zap it!
        game.messages.add(
            format!(
                "A lightning bolt strikes the {} with a loud thunder! \
                 The damage is {} hit points.",
                game.objects[monster_id].name, LIGHTNING_DAMAGE
            ),
            LIGHT_BLUE,
        );
        if let Some(xp) = game.objects[monster_id].take_damage(LIGHTNING_DAMAGE, &mut game.messages)
        {
            game.objects[PLAYER].fighter.as_mut().unwrap().xp += xp;
        }
        UseResult::UsedUp
    } else {
        // no enemy found within maximum range
        game.messages
            .add("No enemy is close enough to strike.", RED);
        UseResult::Cancelled
    }
}

/// find closest enemy, up to a maximum range, and in the player's FOV
pub fn closest_monster(tcod: &Tcod, objects: &[Object], max_range: i32) -> Option<usize> {
    let mut closest_enemy = None;
    let mut closest_dist = (max_range + 1) as f32; // start with (slightly more than) maximum range

    for (id, object) in objects.iter().enumerate() {
        if (id != PLAYER)
            && object.fighter.is_some()
            && object.ai.is_some()
            && tcod.fov.is_in_fov(object.x, object.y)
        {
            // calculate distance between this object and the player
            let dist = objects[PLAYER].distance_to(object);
            if dist < closest_dist {
                // it's closer, so remember it
                closest_enemy = Some(id);
                closest_dist = dist;
            }
        }
    }
    closest_enemy
}

pub fn cast_confuse(_inventory_id: usize, tcod: &mut Tcod, game: &mut Game) -> UseResult {
    // ask the player for a target to confuse
    game.messages.add(
        "Left-click an enemy to confuse it, or right-click to cancel.",
        LIGHT_CYAN,
    );
    let monster_id = target_monster(tcod, game, Some(CONFUSE_RANGE as f32));
    if let Some(monster_id) = monster_id {
        let old_ai = game.objects[monster_id].ai.take().unwrap_or(Ai::Basic);
        // replace the monster's AI with a "confused" one; after
        // some turns it will restore the old AI
        game.objects[monster_id].ai = Some(Ai::Confused {
            previous_ai: Box::new(old_ai),
            num_turns: CONFUSE_NUM_TURNS,
        });
        game.messages.add(
            format!(
                "The eyes of {} look vacant, as he starts to stumble around!",
                game.objects[monster_id].name
            ),
            LIGHT_GREEN,
        );
        UseResult::UsedUp
    } else {
        // no enemy fonud within maximum range
        game.messages
            .add("No enemy is close enough to strike.", RED);
        UseResult::Cancelled
    }
}

/// returns a clicked monster inside FOV up to a range, or None if right-clicked
pub fn target_monster(tcod: &mut Tcod, game: &mut Game, max_range: Option<f32>) -> Option<usize> {
    loop {
        match game.target_tile(tcod, max_range) {
            Some((x, y)) => {
                // return the first clicked monster, otherwise continue looping
                for (id, obj) in game.objects.iter().enumerate() {
                    if obj.pos() == (x, y) && obj.fighter.is_some() && id != PLAYER {
                        return Some(id);
                    }
                }
            }
            None => return None,
        }
    }
}
