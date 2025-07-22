pub mod chrono {
    use chrono::{DateTime, Local, TimeZone, Utc};

    pub fn to_local_date_time(utc_date_time: &DateTime<Utc>) -> DateTime<Local> {
        Local.from_utc_datetime(&utc_date_time.naive_utc())
    }
}

pub mod egui {
    use egui::{Context, Layout};
    
    pub fn item_spacing(ctx: &Context, layout: &Layout) -> f32 {
        if layout.is_vertical() {
            ctx.style().spacing.item_spacing.y
        } else {
            ctx.style().spacing.item_spacing.x
        }
    }
}
