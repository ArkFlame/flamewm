use flamewm_api::applications::DesktopApplication;
use flamewm_api::{OutputId, PanelEdge, Rect};
use flamewm_ui_core::style::ShellMetrics;
use flamewm_ui_core::{PopoverDirection, anchor_popover};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartCategory {
    All,
    Accessories,
    Development,
    Education,
    Games,
    Graphics,
    Internet,
    Multimedia,
    Office,
    Science,
    Settings,
    System,
    Other,
}

impl StartCategory {
    #[must_use]
    pub fn from_application(application: &DesktopApplication) -> Self {
        application
            .categories
            .iter()
            .map(|category| category.to_lowercase())
            .find_map(|category| match category.as_str() {
                "accessories" | "utility" => Some(Self::Accessories),
                "development" => Some(Self::Development),
                "education" => Some(Self::Education),
                "game" | "games" => Some(Self::Games),
                "graphics" => Some(Self::Graphics),
                "network" | "internet" => Some(Self::Internet),
                "audiovideo" | "audio" | "video" | "multimedia" => Some(Self::Multimedia),
                "office" => Some(Self::Office),
                "science" => Some(Self::Science),
                "settings" => Some(Self::Settings),
                "system" => Some(Self::System),
                _ => None,
            })
            .unwrap_or(Self::Other)
    }
    #[must_use]
    fn accepts(self, application: &DesktopApplication) -> bool {
        self == Self::All || self == Self::from_application(application)
    }
}

#[must_use]
pub fn start_category(application: &DesktopApplication) -> StartCategory {
    StartCategory::from_application(application)
}

fn compare_applications(left: &DesktopApplication, right: &DesktopApplication) -> Ordering {
    left.name
        .to_lowercase()
        .cmp(&right.name.to_ascii_lowercase())
        .then_with(|| {
            left.id
                .as_str()
                .to_lowercase()
                .cmp(&right.id.as_str().to_lowercase())
        })
}

#[must_use]
pub fn filter_start_applications<'a>(
    applications: &'a [DesktopApplication],
    category: StartCategory,
    query: &str,
) -> Vec<&'a DesktopApplication> {
    let needle = query.trim().to_lowercase();
    let mut results: Vec<_> = applications
        .iter()
        .filter(|application| category.accepts(application))
        .filter(|application| {
            needle.is_empty()
                || application.name.to_lowercase().contains(&needle)
                || application.id.as_str().to_lowercase().contains(&needle)
                || application.generic_name.to_lowercase().contains(&needle)
                || application.comment.to_lowercase().contains(&needle)
                || application
                    .keywords
                    .iter()
                    .any(|keyword| keyword.to_lowercase().contains(&needle))
                || application
                    .categories
                    .iter()
                    .any(|value| value.to_lowercase().contains(&needle))
        })
        .collect();
    results.sort_by(|left, right| compare_applications(left, right));
    results
}

#[must_use]
pub fn search_start_applications<'a>(
    applications: &'a [DesktopApplication],
    query: &str,
) -> Vec<&'a DesktopApplication> {
    filter_start_applications(applications, StartCategory::All, query)
}

#[must_use]
pub fn project_start_slots<'a>(
    applications: &'a [DesktopApplication],
    first_slot: usize,
    slot_count: usize,
) -> Vec<&'a DesktopApplication> {
    applications
        .iter()
        .skip(first_slot)
        .take(slot_count)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartModel {
    applications: Vec<DesktopApplication>,
    query: String,
}
impl StartModel {
    #[must_use]
    pub fn new(applications: Vec<DesktopApplication>) -> Self {
        Self {
            applications,
            query: String::new(),
        }
    }
    pub fn replace_applications(&mut self, applications: Vec<DesktopApplication>) {
        self.applications = applications;
    }
    #[must_use]
    pub fn applications(&self) -> &[DesktopApplication] {
        &self.applications
    }
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
    }
    #[must_use]
    pub fn results(&self) -> Vec<&DesktopApplication> {
        search_start_applications(&self.applications, &self.query)
    }
    #[must_use]
    pub fn results_for_category(&self, category: StartCategory) -> Vec<&DesktopApplication> {
        filter_start_applications(&self.applications, category, &self.query)
    }
    #[must_use]
    pub fn results_in_slots(
        &self,
        first_slot: usize,
        slot_count: usize,
    ) -> Vec<&DesktopApplication> {
        self.results()
            .into_iter()
            .skip(first_slot)
            .take(slot_count)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartSurfaceLayout {
    pub button: Rect,
    pub anchor: Rect,
    pub popover: Rect,
}

#[must_use]
pub fn start_surface_layout(
    output: OutputId,
    output_rect: Rect,
    edge: PanelEdge,
    metrics: ShellMetrics,
) -> StartSurfaceLayout {
    let (button_w, button_h) = if edge.is_horizontal() {
        (
            i32::from(metrics.start_button_width),
            i32::from(metrics.panel_height),
        )
    } else {
        (
            i32::from(metrics.panel_height),
            i32::from(metrics.start_button_width),
        )
    };
    let anchor = match edge {
        PanelEdge::Bottom => Rect::new(
            output_rect.x,
            output_rect.bottom() - button_h,
            button_w,
            button_h,
        ),
        PanelEdge::Top | PanelEdge::Left => {
            Rect::new(output_rect.x, output_rect.y, button_w, button_h)
        }
        PanelEdge::Right => Rect::new(
            output_rect.right() - button_w,
            output_rect.y,
            button_w,
            button_h,
        ),
    };
    let direction = match edge {
        PanelEdge::Bottom => PopoverDirection::Above,
        PanelEdge::Top => PopoverDirection::Below,
        PanelEdge::Left => PopoverDirection::RightOf,
        PanelEdge::Right => PopoverDirection::LeftOf,
    };
    let popover = anchor_popover(
        output,
        output_rect,
        anchor,
        (
            i32::from(metrics.start_menu_width),
            i32::from(metrics.start_menu_min_height),
        ),
        direction,
        i32::from(metrics.popover_offset),
    )
    .rect;
    StartSurfaceLayout {
        button: Rect::new(0, 0, button_w, button_h),
        anchor,
        popover,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flamewm_api::DesktopAppId;

    fn application(name: &str) -> DesktopApplication {
        DesktopApplication {
            id: DesktopAppId::new("org.example.editor"),
            name: name.to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            startup_wm_class: String::new(),
            argv: Vec::new(),
            keywords: Vec::new(),
            icon_name: String::new(),
            categories: Vec::new(),
        }
    }

    #[test]
    fn search_matches_generic_name_and_comment() {
        let mut app = application("Editor");
        app.generic_name = "Document Writer".to_owned();
        app.comment = "Create and edit text files".to_owned();
        let applications = vec![app];

        assert_eq!(search_start_applications(&applications, "writer").len(), 1);
        assert_eq!(
            search_start_applications(&applications, "TEXT FILES").len(),
            1
        );
    }

    #[test]
    fn empty_search_returns_all_sorted_applications() {
        let applications = vec![application("Zulu"), application("Alpha")];
        let results = search_start_applications(&applications, "  ");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].name, "Alpha");
        assert_eq!(results[1].name, "Zulu");
    }
}
