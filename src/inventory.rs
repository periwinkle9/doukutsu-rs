use crate::engine_constants::EngineConstants;
use crate::SharedGameState;
use crate::weapon::{Weapon, WeaponLevel, WeaponType};

#[derive(Clone, Copy)]
pub struct Item(u16);

#[derive(Clone)]
pub struct Inventory {
    current_item: u16,
    current_weapon: u16,
    items: Vec<Item>,
    weapons: Vec<Weapon>,
}

#[derive(Clone, Copy)]
pub enum AddExperienceResult {
    None,
    LevelUp,
    AddStar,
}

impl Inventory {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Inventory {
        Inventory {
            current_item: 0,
            current_weapon: 0,
            items: Vec::with_capacity(16),
            weapons: Vec::with_capacity(16),
        }
    }

    pub fn add_item(&mut self, item_id: u16) {
        if !self.has_item(item_id) {
            self.items.push(Item(item_id));
        }
    }

    pub fn remove_item(&mut self, item_id: u16) {
        self.items.retain(|item| item.0 != item_id);
    }

    pub fn has_item(&self, item_id: u16) -> bool {
        self.items.iter().any(|item| item.0 == item_id)
    }

    pub fn add_weapon(&mut self, weapon_id: WeaponType, max_ammo: u16) {
        if !self.has_weapon(weapon_id) {
            self.weapons.push(Weapon::new(
                weapon_id,
                WeaponLevel::Level1,
                0,
                max_ammo,
                max_ammo,
            ));
        }
    }

    pub fn remove_weapon(&mut self, wtype: WeaponType) {
        self.weapons.retain(|weapon| weapon.wtype != wtype);
    }

    pub fn get_weapon(&self, idx: usize) -> Option<&Weapon> {
        self.weapons.get(idx)
    }

    pub fn get_current_weapon(&self) -> Option<&Weapon> {
        self.weapons.get(self.current_weapon as usize)
    }

    pub fn get_current_weapon_mut(&mut self) -> Option<&mut Weapon> {
        self.weapons.get_mut(self.current_weapon as usize)
    }

    pub fn refill_all_ammo(&mut self) {
        for weapon in self.weapons.iter_mut() {
            weapon.ammo = weapon.max_ammo;
        }
    }

    pub fn reset_all_weapon_xp(&mut self) {
        for weapon in self.weapons.iter_mut() {
            weapon.level = WeaponLevel::Level1;
            weapon.experience = 0;
        }
    }

    pub fn add_xp(&mut self, exp: u16, state: &mut SharedGameState) -> AddExperienceResult {
        let mut result = AddExperienceResult::None;

        if let Some(weapon) = self.get_current_weapon_mut() {
            let curr_level_idx = weapon.level as usize - 1;
            let lvl_table = state.constants.weapon.level_table[weapon.wtype as usize];

            weapon.experience += exp;

            if weapon.level == WeaponLevel::Level3 {
                if weapon.experience > lvl_table[2] {
                    weapon.experience = lvl_table[2];
                    result = AddExperienceResult::AddStar;
                }
            } else {
                if weapon.experience > lvl_table[curr_level_idx] {
                    weapon.level = weapon.level.next();
                    weapon.experience = 0;

                    if weapon.wtype != WeaponType::Spur {
                        result = AddExperienceResult::LevelUp;
                    }
                }
            }
        }

        result
    }

    /// Get current experience state. Returns a (exp, max exp, max level/exp) tuple.
    pub fn get_current_max_exp(&self, constants: &EngineConstants) -> (u16, u16, bool) {
        if let Some(weapon) = self.weapons.get(self.current_weapon as usize) {
            if weapon.level == WeaponLevel::None {
                return (0, 0, false);
            }

            let level_idx = weapon.level as usize - 1;
            let max_exp = constants.weapon.level_table[weapon.wtype as usize][level_idx];
            let max = weapon.level == WeaponLevel::Level3 && weapon.experience == max_exp;

            (weapon.experience, max_exp, max)
        } else {
            (0, 0, false)
        }
    }

    /// Get current ammunition state. Returns a (ammo, max ammo) tuple.
    pub fn get_current_ammo(&self) -> (u16, u16) {
        if let Some(weapon) = self.weapons.get(self.current_weapon as usize) {
            (weapon.ammo, weapon.max_ammo)
        } else {
            (0, 0)
        }
    }

    pub fn get_current_level(&self) -> WeaponLevel {
        if let Some(weapon) = self.weapons.get(self.current_weapon as usize) {
            weapon.level
        } else {
            WeaponLevel::None
        }
    }

    pub fn get_current_weapon_idx(&self) -> u16 {
        self.current_weapon
    }

    pub fn get_weapon_count(&self) -> usize {
        self.weapons.len()
    }

    pub fn has_weapon(&self, wtype: WeaponType) -> bool {
        self.weapons.iter().any(|weapon| weapon.wtype == wtype)
    }
}