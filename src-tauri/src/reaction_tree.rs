use serde::Serialize;
use std::collections::{HashMap, HashSet};

use crate::prices::PriceInfo;
use crate::reactions::ReactionDatabase;

/// Check if an item can be traced back to user's moon materials
/// Either directly (it IS a moon material) or indirectly (it can be made from reactions
/// that eventually use moon materials)
pub fn traces_to_moon_materials(
    item_id: u32,
    reactions_db: &ReactionDatabase,
    user_moon_goo_ids: &HashSet<u32>,
    visited: &mut HashSet<u32>,
) -> bool {
    // If this is directly a moon material, return true
    if user_moon_goo_ids.contains(&item_id) {
        return true;
    }

    // Prevent infinite loops
    if visited.contains(&item_id) {
        return false;
    }
    visited.insert(item_id);

    // Check if this item can be produced by a reaction
    if let Some(reaction) = reactions_db.by_output.get(&item_id) {
        // Check if ANY of the inputs trace back to moon materials
        for input in &reaction.inputs {
            if traces_to_moon_materials(input.id, reactions_db, user_moon_goo_ids, visited) {
                return true;
            }
        }
    }

    false
}

/// Check if a reaction uses user materials (directly or indirectly through the chain)
pub fn reaction_uses_user_materials(
    reaction: &crate::reactions::Reaction,
    reactions_db: &ReactionDatabase,
    user_moon_goo_ids: &HashSet<u32>,
) -> bool {
    let mut visited = HashSet::new();
    for input in &reaction.inputs {
        if traces_to_moon_materials(input.id, reactions_db, user_moon_goo_ids, &mut visited) {
            return true;
        }
    }
    false
}

/// Source type for a material in the reaction tree
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Moon,   // Comes from user's moons
    Buy,    // Must be purchased
    React,  // Produced by running a reaction
    Output, // Final output (sell this)
}

/// A node in the reaction tree
#[derive(Debug, Clone, Serialize)]
pub struct ReactionTreeNode {
    pub name: String,
    pub id: u32,
    pub quantity: u32,
    pub source: SourceType,
    pub unit_price: f64,
    pub total_price: f64,
    /// For REACT nodes, the reaction that produces this
    pub reaction_name: Option<String>,
    /// Child nodes (inputs needed to produce this)
    pub children: Vec<ReactionTreeNode>,
}

/// Build a reaction tree for a given output item
pub fn build_reaction_tree(
    item_name: &str,
    item_id: u32,
    quantity: u32,
    reactions_db: &ReactionDatabase,
    user_moon_goo_ids: &HashSet<u32>,
    prices: &HashMap<String, PriceInfo>,
    visited: &mut HashSet<u32>, // Prevent infinite loops
) -> ReactionTreeNode {
    let unit_price = prices.get(item_name).map(|p| p.sell).unwrap_or(0.0);
    let total_price = unit_price * quantity as f64;

    // Check if this is from user's moons
    if user_moon_goo_ids.contains(&item_id) {
        return ReactionTreeNode {
            name: item_name.to_string(),
            id: item_id,
            quantity,
            source: SourceType::Moon,
            unit_price,
            total_price,
            reaction_name: None,
            children: vec![],
        };
    }

    // Check if this can be produced by a reaction (and we haven't visited it yet)
    if let Some(reaction) = reactions_db.by_output.get(&item_id) {
        if !visited.contains(&item_id) {
            visited.insert(item_id);

            // Calculate how many reaction runs we need
            let runs_needed = (quantity as f64 / reaction.output.quantity as f64).ceil() as u32;

            // Build child nodes for each input
            let children: Vec<ReactionTreeNode> = reaction
                .inputs
                .iter()
                .map(|input| {
                    let input_quantity = input.quantity * runs_needed;
                    build_reaction_tree(
                        &input.name,
                        input.id,
                        input_quantity,
                        reactions_db,
                        user_moon_goo_ids,
                        prices,
                        visited,
                    )
                })
                .collect();

            visited.remove(&item_id); // Allow this item to be visited in other branches

            return ReactionTreeNode {
                name: item_name.to_string(),
                id: item_id,
                quantity,
                source: SourceType::React,
                unit_price,
                total_price,
                reaction_name: Some(reaction.formula_name.clone()),
                children,
            };
        }
    }

    // If not from moon and not reactable, it must be bought
    ReactionTreeNode {
        name: item_name.to_string(),
        id: item_id,
        quantity,
        source: SourceType::Buy,
        unit_price,
        total_price,
        reaction_name: None,
        children: vec![],
    }
}

/// Build the full tree for a profitable reaction output
pub fn build_full_reaction_tree(
    output_name: &str,
    output_id: u32,
    output_quantity: u32,
    reactions_db: &ReactionDatabase,
    user_moon_goo_ids: &HashSet<u32>,
    prices: &HashMap<String, PriceInfo>,
) -> ReactionTreeNode {
    let unit_price = prices.get(output_name).map(|p| p.sell).unwrap_or(0.0);
    let total_price = unit_price * output_quantity as f64;

    // Get the reaction for this output
    let reaction = reactions_db.by_output.get(&output_id);

    let children = if let Some(reaction) = reaction {
        let mut visited = HashSet::new();
        visited.insert(output_id); // Mark output as visited to prevent loops

        reaction
            .inputs
            .iter()
            .map(|input| {
                build_reaction_tree(
                    &input.name,
                    input.id,
                    input.quantity,
                    reactions_db,
                    user_moon_goo_ids,
                    prices,
                    &mut visited,
                )
            })
            .collect()
    } else {
        vec![]
    };

    ReactionTreeNode {
        name: output_name.to_string(),
        id: output_id,
        quantity: output_quantity,
        source: SourceType::Output,
        unit_price,
        total_price,
        reaction_name: reaction.map(|r| r.formula_name.clone()),
        children,
    }
}
