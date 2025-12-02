use iced::widget::container;
use iced::{Color, Theme};

#[derive(Clone, Copy, Default)]
pub enum ContainerTheme {
    #[default]
    Odd,
    Even,
}

impl container::StyleSheet for ContainerTheme {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        match self {
            ContainerTheme::Odd => container::Appearance {
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            },
            ContainerTheme::Even => container::Appearance {
                background: Some(Color::from_rgb(0.95, 0.95, 0.95).into()),
                ..Default::default()
            },
        }
    }
}
