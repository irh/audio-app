mod app;
mod widgets;

use crate::app::App;
use anyhow::Result;
use freeverb_module::FreeverbParameters;
use iced::{Size, Task};
use log::LevelFilter;

pub fn main() -> Result<()> {
    pretty_env_logger::formatted_builder()
        .filter_level(LevelFilter::Info)
        .init();

    iced::application(env!("PRODUCT_NAME"), App::update, App::view)
        .subscription(App::subscription)
        .window_size(Size::new(760.0, 500.0))
        .run_with(move || {
            let initial_state = App::new(FreeverbParameters::default());
            (initial_state, Task::none())
        })?;

    Ok(())
}
