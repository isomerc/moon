use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInfo {
    pub buy: f64,
    pub sell: f64,
}

#[derive(Debug, Deserialize)]
struct AppraisalItem {
    #[serde(rename = "typeName")]
    type_name: String,
    prices: AppraisalPrices,
}

#[derive(Debug, Deserialize)]
struct AppraisalPrices {
    buy: PriceDetail,
    sell: PriceDetail,
}

#[derive(Debug, Deserialize)]
struct PriceDetail {
    percentile: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct AppraisalInner {
    items: Vec<AppraisalItem>,
}

#[derive(Debug, Deserialize)]
struct AppraisalResponse {
    appraisal: AppraisalInner,
}

/// Fetch prices for a list of item names from Goonpraisal
pub async fn fetch_prices(item_names: &[String]) -> Result<HashMap<String, PriceInfo>, String> {
    if item_names.is_empty() {
        return Ok(HashMap::new());
    }

    let client = reqwest::Client::new();

    // Build the request body - one item per line
    let raw_textarea = item_names.join("\n");

    let response = client
        .post("https://appraise.gnf.lt/appraisal.json")
        .header("User-Agent", "MOON-Reaction-Calculator/1.0")
        .form(&[
            ("market", "jita"),
            ("raw_textarea", &raw_textarea),
            ("persist", "no"),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to fetch prices: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Goonpraisal returned status: {}",
            response.status()
        ));
    }

    let appraisal: AppraisalResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse price response: {}", e))?;

    let mut prices = HashMap::new();
    for item in appraisal.appraisal.items {
        prices.insert(
            item.type_name,
            PriceInfo {
                buy: item.prices.buy.percentile.unwrap_or(0.0),
                sell: item.prices.sell.percentile.unwrap_or(0.0),
            },
        );
    }

    Ok(prices)
}

#[derive(Debug, Clone, Serialize)]
pub struct InputBreakdown {
    pub name: String,
    pub quantity: u32,
    pub unit_price: f64,
    pub total_price: f64,
    pub from_moon: bool, // true if user has this from their moons (but still has opportunity cost)
}

#[derive(Debug, Clone, Serialize)]
pub struct ReactionProfit {
    pub formula_id: u32,
    pub formula_name: String,
    pub output_name: String,
    pub output_id: u32,
    pub output_quantity: u32,
    pub output_unit_price: f64,
    pub output_value: f64,
    pub input_cost: f64, // Total opportunity cost of all inputs (sell value)
    pub profit: f64,
    pub margin: f64,
    pub inputs: Vec<InputBreakdown>,
    pub uses_user_materials: bool, // true if at least one input is from user's moons
    pub reaction_tree: Option<crate::reaction_tree::ReactionTreeNode>,
}

/// Calculate profit for a reaction (inputs priced at sell value for opportunity cost)
pub fn calculate_reaction_profit(
    reaction: &crate::reactions::Reaction,
    prices: &HashMap<String, PriceInfo>,
    user_material_ids: &HashSet<u32>,
) -> Option<ReactionProfit> {
    let output_price = prices.get(&reaction.output.name)?;
    let output_unit_price = output_price.sell;
    let output_value = output_unit_price * reaction.output.quantity as f64;

    let mut input_cost = 0.0;
    let mut inputs = Vec::new();
    let mut uses_user_materials = false;

    for input in &reaction.inputs {
        let input_price = prices.get(&input.name)?;
        let unit_price = input_price.sell;
        let from_moon = user_material_ids.contains(&input.id);

        if from_moon {
            uses_user_materials = true;
        }

        let total_price = unit_price * input.quantity as f64;
        input_cost += total_price;

        inputs.push(InputBreakdown {
            name: input.name.clone(),
            quantity: input.quantity,
            unit_price,
            total_price,
            from_moon,
        });
    }

    let profit = output_value - input_cost;
    let margin = if input_cost > 0.0 {
        (profit / input_cost) * 100.0
    } else {
        0.0
    };

    Some(ReactionProfit {
        formula_id: reaction.formula_id,
        formula_name: reaction.formula_name.clone(),
        output_name: reaction.output.name.clone(),
        output_id: reaction.output.id,
        output_quantity: reaction.output.quantity,
        output_unit_price,
        output_value,
        input_cost,
        profit,
        margin,
        inputs,
        uses_user_materials,
        reaction_tree: None, // Will be populated separately
    })
}
