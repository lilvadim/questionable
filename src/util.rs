use std::collections::HashMap;

use regex::Regex;

pub mod chrono {
    use chrono::{DateTime, Local, TimeZone, Utc};

    #[inline]
    pub fn to_local_date_time(utc_date_time: &DateTime<Utc>) -> DateTime<Local> {
        Local.from_utc_datetime(&utc_date_time.naive_utc())
    }
}

pub mod egui {
    use egui::{Context, Layout};

    #[inline]
    pub fn item_spacing(ctx: &Context, layout: &Layout) -> f32 {
        if layout.is_vertical() {
            ctx.style().spacing.item_spacing.y
        } else {
            ctx.style().spacing.item_spacing.x
        }
    }
}

pub fn generate_unique_name<'x>(
    existing_names: impl IntoIterator<Item = &'x str>,
    candidate_name: String,
) -> String {
    let start_count = 1;
    let counts = existing_names
        .into_iter()
        .filter_map(|existing_name| {
            Regex::new(&format!(r"^{candidate_name}( #(?<count>\d+))*$"))
                .unwrap()
                .captures(existing_name)
        })
        .map(|caps| {
            caps.name("count")
                .and_then(|c| c.as_str().parse::<i32>().ok())
                .unwrap_or(start_count)
        })
        .map(|count| (count, true))
        .collect::<HashMap<i32, bool>>();
    let max_count = counts.keys().max().cloned().unwrap_or(-1);
    for i in start_count..=max_count + 1 {
        if counts.get(&i).is_none() {
            return if i == 0 {
                candidate_name
            } else {
                format!("{candidate_name} #{i}")
            };
        }
    }

    candidate_name
}
