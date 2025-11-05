mod app;
mod widgets;

use crate::app::{App, UI_SIZE};
use anyhow::Result;
use freeverb_module::FreeverbParameters;
use vizia::prelude::*;

fn main() -> Result<()> {
    Application::new(move |cx| {
        App::build(cx, FreeverbParameters::default());
    })
    .title(env!("PRODUCT_NAME"))
    .inner_size(UI_SIZE)
    .run()?;

    Ok(())
}
