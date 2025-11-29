use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
struct OreMappingsFile {
    #[serde(rename = "R4_Ubiquitous")]
    r4: HashMap<String, HashMap<String, u32>>,
    #[serde(rename = "R8_Common")]
    r8: HashMap<String, HashMap<String, u32>>,
    #[serde(rename = "R16_Uncommon")]
    r16: HashMap<String, HashMap<String, u32>>,
    #[serde(rename = "R32_Rare")]
    r32: HashMap<String, HashMap<String, u32>>,
    #[serde(rename = "R64_Exceptional")]
    r64: HashMap<String, HashMap<String, u32>>,
}

/// Known ore variant prefixes that should be stripped to get base ore name
const ORE_PREFIXES: &[&str] = &[
    "Bountiful ",
    "Copious ",
    "Dazzling ",
    "Flawless ",
    "Gilded ",
    "Glossy ",
    "Immaculate ",
    "Lavish ",
    "Lustrous ",
    "Opulent ",
    "Pellucid ",
    "Platelet ",
    "Plentiful ",
    "Prismatic ",
    "Radiant ",
    "Replete ",
    "Resplendent ",
    "Shimmering ",
    "Sparkling ",
    "Stable ",
    "Twinkling ",
    "Brilliant ",
];

pub struct OreMappings {
    /// Map from base ore name -> list of moon goo materials it produces
    ore_to_goo: HashMap<String, Vec<String>>,
}

impl OreMappings {
    pub fn load() -> Result<Self, String> {
        let json_str = include_str!("../mappings.json");
        let mappings: OreMappingsFile = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse mappings: {}", e))?;

        let mut ore_to_goo: HashMap<String, Vec<String>> = HashMap::new();

        // Combine all tiers
        for tier in [
            mappings.r4,
            mappings.r8,
            mappings.r16,
            mappings.r32,
            mappings.r64,
        ] {
            for (ore_name, materials) in tier {
                let goo_materials: Vec<String> = materials
                    .keys()
                    .filter(|name| is_moon_goo(name))
                    .cloned()
                    .collect();
                ore_to_goo.insert(ore_name, goo_materials);
            }
        }

        Ok(Self { ore_to_goo })
    }

    /// Strip variant prefix from ore name to get base ore
    pub fn get_base_ore_name(ore_name: &str) -> String {
        for prefix in ORE_PREFIXES {
            if let Some(stripped) = ore_name.strip_prefix(prefix) {
                return stripped.to_string();
            }
        }
        ore_name.to_string()
    }

    /// Given a list of ore names from moon scans, return the set of moon goo materials
    pub fn ores_to_moon_goo(&self, ore_names: &[String]) -> HashSet<String> {
        let mut goo_materials = HashSet::new();

        for ore_name in ore_names {
            let base_ore = Self::get_base_ore_name(ore_name);
            if let Some(materials) = self.ore_to_goo.get(&base_ore) {
                for mat in materials {
                    goo_materials.insert(mat.clone());
                }
            }
        }

        goo_materials
    }
}

/// Check if a material name is moon goo (used in reactions) vs regular minerals
fn is_moon_goo(name: &str) -> bool {
    matches!(
        name,
        // R4 moon goo
        "Hydrocarbons" | "Silicates" | "Evaporite Deposits" | "Atmospheric Gases" |
        // R8 moon goo
        "Cobalt" | "Scandium" | "Tungsten" | "Titanium" |
        // R16 moon goo
        "Chromium" | "Cadmium" | "Platinum" | "Vanadium" |
        // R32 moon goo
        "Technetium" | "Mercury" | "Caesium" | "Hafnium" |
        // R64 moon goo
        "Promethium" | "Neodymium" | "Dysprosium" | "Thulium"
    )
}
