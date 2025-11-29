use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoonComposition {
    pub name: String,
    pub materials: Vec<MaterialEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialEntry {
    pub name: String,
    pub quantity: f64,
    pub item_id: u32,
    pub system_id: u32,
    pub region_id: u32,
    pub additional_id: u32,
}

#[derive(Debug)]
pub enum ParseError {
    InvalidFormat(String),
    InvalidNumber(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ParseError::InvalidNumber(msg) => write!(f, "Invalid number: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

pub fn parse_moon_data(input: &str) -> Result<Vec<MoonComposition>, ParseError> {
    let mut moons = Vec::new();
    let mut current_moon: Option<MoonComposition> = None;

    for line in input.lines() {
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        // Count leading whitespace to distinguish moon names from materials
        // Material entries have more indentation (typically 4+ spaces)
        // Moon names have less indentation (typically 0-1 spaces)
        let leading_spaces = line.chars().take_while(|c| c.is_whitespace()).count();

        if leading_spaces >= 4 {
            // Material entry
            if let Some(ref mut moon) = current_moon {
                let material = parse_material_line(line)?;
                moon.materials.push(material);
            } else {
                return Err(ParseError::InvalidFormat(
                    "Material entry found before moon name".to_string(),
                ));
            }
        } else {
            // Moon name - save previous moon if exists
            if let Some(moon) = current_moon.take() {
                if moon.materials.is_empty() {
                    return Err(ParseError::InvalidFormat(format!(
                        "Moon '{}' has no materials",
                        moon.name
                    )));
                }
                moons.push(moon);
            }

            // Start new moon
            current_moon = Some(MoonComposition {
                name: line.trim().to_string(),
                materials: Vec::new(),
            });
        }
    }

    // Handle last moon
    if let Some(moon) = current_moon {
        if moon.materials.is_empty() {
            return Err(ParseError::InvalidFormat(format!(
                "Moon '{}' has no materials",
                moon.name
            )));
        }
        moons.push(moon);
    }

    // Ensure we parsed at least one moon
    if moons.is_empty() {
        return Err(ParseError::InvalidFormat(
            "No valid moon data found".to_string(),
        ));
    }

    Ok(moons)
}

fn parse_material_line(line: &str) -> Result<MaterialEntry, ParseError> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 6 {
        return Err(ParseError::InvalidFormat(format!(
            "Expected at least 6 fields, got {}",
            parts.len()
        )));
    }

    // Material name might be multiple words, so we need to find where the numbers start
    // The format is: MaterialName Percentage ID1 ID2 ID3 ID4
    // We know the last 5 fields are numbers, so everything before that is the name
    let num_count = 5; // percentage + 4 IDs
    let name_parts = &parts[..parts.len() - num_count];
    let number_parts = &parts[parts.len() - num_count..];

    let name = name_parts.join(" ");

    let quantity = number_parts[0]
        .parse::<f64>()
        .map_err(|e| ParseError::InvalidNumber(format!("Invalid quantity: {}", e)))?;

    let item_id = number_parts[1]
        .parse::<u32>()
        .map_err(|e| ParseError::InvalidNumber(format!("Invalid item_id: {}", e)))?;

    let system_id = number_parts[2]
        .parse::<u32>()
        .map_err(|e| ParseError::InvalidNumber(format!("Invalid system_id: {}", e)))?;

    let region_id = number_parts[3]
        .parse::<u32>()
        .map_err(|e| ParseError::InvalidNumber(format!("Invalid region_id: {}", e)))?;

    let additional_id = number_parts[4]
        .parse::<u32>()
        .map_err(|e| ParseError::InvalidNumber(format!("Invalid additional_id: {}", e)))?;

    Ok(MaterialEntry {
        name,
        quantity,
        item_id,
        system_id,
        region_id,
        additional_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_example() {
        let input = r#" OP9L-F II - Moon 1
    Glossy Scordite 0.300030559301  46687   30002173    40138526    40138527
    Immaculate Jaspet   0.328855156898  46682   30002173    40138526    40138527
    Pellucid Crokite    0.287893354893  46677   30002173    40138526    40138527
    Sylvite 0.083220936358  45491   30002173    40138526    40138527
"#;

        let moons = parse_moon_data(input).unwrap();

        assert_eq!(moons.len(), 1);
        assert_eq!(moons[0].name, "OP9L-F II - Moon 1");
        assert_eq!(moons[0].materials.len(), 4);

        let first_material = &moons[0].materials[0];
        assert_eq!(first_material.name, "Glossy Scordite");
        assert!((first_material.quantity - 0.300030559301).abs() < 0.0001);
        assert_eq!(first_material.item_id, 46687);
    }

    #[test]
    fn test_reject_arbitrary_text() {
        let input = "some random text without proper format";
        let result = parse_moon_data(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_moon_without_materials() {
        let input = "Moon Name\nAnother Moon Name";
        let result = parse_moon_data(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_empty_input() {
        let input = "";
        let result = parse_moon_data(input);
        assert!(result.is_err());
    }
}
