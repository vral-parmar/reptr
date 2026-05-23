use anyhow::Result;

use crate::model::Engagement;

pub fn render(engagement: &Engagement) -> Result<String> {
    Ok(serde_json::to_string_pretty(engagement)?)
}
