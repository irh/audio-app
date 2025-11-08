mod app;
pub use app::App;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use eframe::{NativeOptions, run_native};
    use log::LevelFilter;

    android_logger::init_once(android_logger::Config::default().with_max_level(LevelFilter::Info));

    run_native(
        env!("PRODUCT_NAME"),
        NativeOptions {
            android_app: Some(app),
            ..Default::default()
        },
        Box::new(|_| Ok(Box::new(App::new()?))),
    )
    .unwrap()
}
