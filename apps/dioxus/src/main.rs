mod components;

use dioxus::prelude::*;
use self::components::App;

fn main() {
    dioxus::logger::initialize_default();

    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("freeverb")
                .with_filter(
                    android_logger::FilterBuilder::new()
                        .parse("debug,hello::crate=error")
                        .build(),
                ),
        );
    }

    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
            Config::new().with_window(
                WindowBuilder::new()
                   .with_title(env!("PRODUCT_NAME"))
                   .with_inner_size(LogicalSize::new(310, 500))
            )
        })
        .launch(App);
}
