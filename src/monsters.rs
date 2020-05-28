use crate::{
    ai::Ai,
    fighter::{DeathCallback, Fighter},
    object::Object,
};
use rand::distributions::{IndependentSample, Weighted, WeightedChoice};
use tcod::colors;

pub fn create_monster(x: i32, y: i32) -> Object {
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
    monster
}
