use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionItem {
    pub name: String,
    pub id: u32,
    pub quantity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub formula_id: u32,
    pub formula_name: String,
    pub output: ReactionItem,
    pub inputs: Vec<ReactionItem>,
}

/// Loaded reactions database
pub struct ReactionDatabase {
    pub reactions: Vec<Reaction>,
    /// Map from output item ID to reaction
    pub by_output: HashMap<u32, Reaction>,
    /// Map from item name to item ID
    pub name_to_id: HashMap<String, u32>,
}

impl ReactionDatabase {
    pub fn load() -> Result<Self, String> {
        let json_str = include_str!("../reactions.json");
        let reactions: Vec<Reaction> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse reactions: {}", e))?;

        let mut by_output = HashMap::new();
        let mut name_to_id = HashMap::new();

        for reaction in &reactions {
            by_output.insert(reaction.output.id, reaction.clone());
            name_to_id.insert(reaction.output.name.clone(), reaction.output.id);

            for input in &reaction.inputs {
                name_to_id.insert(input.name.clone(), input.id);
            }
        }

        Ok(Self {
            reactions,
            by_output,
            name_to_id,
        })
    }

    /// Get all unique item names needed for price lookups
    pub fn get_all_item_names(&self) -> Vec<String> {
        let mut names: HashSet<String> = HashSet::new();
        for reaction in &self.reactions {
            names.insert(reaction.output.name.clone());
            for input in &reaction.inputs {
                names.insert(input.name.clone());
            }
        }
        names.into_iter().collect()
    }

    /// Get the set of user's free materials (by ID)
    pub fn get_user_material_ids(&self, material_names: &[String]) -> HashSet<u32> {
        material_names
            .iter()
            .filter_map(|name| self.name_to_id.get(name).copied())
            .collect()
    }
}
