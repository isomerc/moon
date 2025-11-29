use std::collections::HashSet;
use std::sync::Mutex;
use tauri::{Manager, State};

mod ore_mappings;
mod parser;
mod prices;
mod reaction_tree;
mod reactions;
mod telemetry;

use ore_mappings::OreMappings;
use prices::ReactionProfit;
use reactions::ReactionDatabase;

// State to hold the loaded moons and reactions
pub struct AppState {
    moons: Mutex<Vec<parser::MoonComposition>>,
    reactions_db: ReactionDatabase,
    ore_mappings: OreMappings,
}

// Parse moon scan data
#[tauri::command]
fn parse_moon_data(input: String) -> Result<Vec<parser::MoonComposition>, String> {
    parser::parse_moon_data(&input).map_err(|e| e.to_string())
}

// Add moon(s) to the state
#[tauri::command]
fn add_moon(
    moons_to_add: Vec<parser::MoonComposition>,
    state: State<AppState>,
) -> Result<(), String> {
    let mut moons = state
        .moons
        .lock()
        .map_err(|_| "Internal error: database lock failed".to_string())?;

    // Check for duplicates
    for new_moon in &moons_to_add {
        if moons.iter().any(|m| m.name == new_moon.name) {
            return Err(format!("Moon '{}' already exists", new_moon.name));
        }
    }

    for moon in moons_to_add {
        moons.push(moon);
    }

    Ok(())
}

// Delete moon by index
#[tauri::command]
fn delete_moon(index: usize, state: State<AppState>) -> Result<(), String> {
    let mut moons = state
        .moons
        .lock()
        .map_err(|_| "Internal error: database lock failed".to_string())?;

    if index >= moons.len() {
        return Err("Invalid moon index".to_string());
    }

    moons.remove(index);
    Ok(())
}

// Get all moons
#[tauri::command]
fn get_moons(state: State<AppState>) -> Result<Vec<parser::MoonComposition>, String> {
    let moons = state
        .moons
        .lock()
        .map_err(|_| "Internal error: database lock failed".to_string())?;
    Ok(moons.clone())
}

// Get unique materials across all moons
#[tauri::command]
fn get_unique_materials(state: State<AppState>) -> Result<Vec<String>, String> {
    let moons = state
        .moons
        .lock()
        .map_err(|_| "Internal error: database lock failed".to_string())?;
    let mut unique_materials: HashSet<String> = HashSet::new();

    for moon in moons.iter() {
        for material in &moon.materials {
            unique_materials.insert(material.name.clone());
        }
    }

    let mut materials_vec: Vec<String> = unique_materials.into_iter().collect();
    materials_vec.sort();
    Ok(materials_vec)
}

// Analyze reactions and find profitable ones based on available moon materials
#[tauri::command]
async fn analyze_reactions(state: State<'_, AppState>) -> Result<Vec<ReactionProfit>, String> {
    // Get ore names from loaded moons
    let ore_names: Vec<String> = {
        let moons = state
            .moons
            .lock()
            .map_err(|_| "Internal error: database lock failed".to_string())?;
        let mut ores: HashSet<String> = HashSet::new();
        for moon in moons.iter() {
            for material in &moon.materials {
                ores.insert(material.name.clone());
            }
        }
        ores.into_iter().collect()
    };

    if ore_names.is_empty() {
        return Err("No moons loaded. Add some moons first.".to_string());
    }

    // Convert ore names to moon goo materials (this is what reactions actually use)
    let moon_goo: HashSet<String> = state.ore_mappings.ores_to_moon_goo(&ore_names);

    if moon_goo.is_empty() {
        return Err(
            "No valid moon ores found. Make sure you're pasting moon scan data.".to_string(),
        );
    }

    // Get the IDs of user's moon materials (to mark which reactions use their materials)
    let moon_goo_vec: Vec<String> = moon_goo.into_iter().collect();
    let user_material_ids = state.reactions_db.get_user_material_ids(&moon_goo_vec);

    // Get ALL item names for price lookup
    let all_items = state.reactions_db.get_all_item_names();

    // Fetch prices from Goonpraisal
    let prices = prices::fetch_prices(&all_items).await?;

    // Calculate profit for each reaction (inputs priced at sell value = opportunity cost)
    let mut profits: Vec<ReactionProfit> = state
        .reactions_db
        .reactions
        .iter()
        .filter(|r| {
            reaction_tree::reaction_uses_user_materials(r, &state.reactions_db, &user_material_ids)
        })
        .filter_map(|r| prices::calculate_reaction_profit(r, &prices, &user_material_ids))
        .filter(|p| p.profit > 0.0)
        .collect();

    for profit in &mut profits {
        let tree = reaction_tree::build_full_reaction_tree(
            &profit.output_name,
            profit.output_id,
            profit.output_quantity,
            &state.reactions_db,
            &user_material_ids,
            &prices,
        );
        profit.reaction_tree = Some(tree);
    }

    profits.sort_by(|a, b| {
        b.margin
            .partial_cmp(&a.margin)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(profits)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix for WebKitGTK on certain Linux/Wayland systems
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        // Set app ID for Wayland taskbar icon
        std::env::set_var("GDK_BACKEND", "x11");
    }

    let reactions_db = ReactionDatabase::load().expect("Failed to load reactions database");
    let ore_mappings = OreMappings::load().expect("Failed to load ore mappings");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            moons: Mutex::new(Vec::new()),
            reactions_db,
            ore_mappings,
        })
        .setup(|app| {
            // Set window icon for Linux/Wayland
            if let Some(window) = app.get_webview_window("main") {
                let icon = app.default_window_icon().cloned();
                if let Some(icon) = icon {
                    let _ = window.set_icon(icon);
                }
            }

            // Send telemetry ping on launch
            telemetry::send_launch_ping();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            parse_moon_data,
            add_moon,
            delete_moon,
            get_moons,
            get_unique_materials,
            analyze_reactions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
