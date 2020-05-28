use crate::{messages::Messages, object::Object};
use serde::{Deserialize, Serialize};
use tcod::colors::{DARK_RED, ORANGE, RED};

// combat-related properties and methods (monster, player, NPC).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fighter {
    pub max_hp: i32,
    pub hp: i32,
    pub defense: i32,
    pub power: i32,
    pub xp: i32,
    pub on_death: DeathCallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DeathCallback {
    Player,
    Monster,
}

impl DeathCallback {
    pub fn callback(self, object: &mut Object, messages: &mut Messages) {
        use DeathCallback::*;
        let callback: fn(&mut Object, messages: &mut Messages) = match self {
            Player => player_death,
            Monster => monster_death,
        };
        callback(object, messages);
    }
}

fn player_death(player: &mut Object, messages: &mut Messages) {
    // the game ended!
    messages.add("You died!", RED);

    // for added effect, transform the player into a corpse!
    player.char = '%';
    player.color = DARK_RED;
}

fn monster_death(monster: &mut Object, messages: &mut Messages) {
    // transform it into a nasty corpse! it doesn't block, can't be
    // attacked and doesn't move
    messages.add(
        format!(
            "{} is dead! You gain {} experience points.",
            monster.name,
            monster.fighter.unwrap().xp
        ),
        ORANGE,
    );
    monster.char = '%';
    monster.color = DARK_RED;
    monster.blocks = false;
    monster.fighter = None;
    monster.ai = None;
    monster.name = format!("remains of {}", monster.name);
}
