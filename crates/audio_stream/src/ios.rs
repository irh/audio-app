#![cfg(target_os = "ios")]

use log::{error, info};

pub fn setup_audio_session() {
    use objc2_avf_audio::{
        AVAudioSession, AVAudioSessionCategoryOptions as Options,
        AVAudioSessionCategoryPlayAndRecord,
    };

    // SAFETY: Calling into system frameworks requires `unsafe`
    unsafe {
        let session = AVAudioSession::sharedInstance();
        let category = AVAudioSessionCategoryPlayAndRecord.unwrap();
        let options = Options::MixWithOthers | Options::AllowBluetoothHFP;

        match session.setCategory_withOptions_error(category, options) {
            Ok(_) => match session.setActive_error(true) {
                Ok(_) => info!("AVAudioSession activated"),
                Err(error) => {
                    error!("error while activating AVAudioSession: {error}");
                }
            },
            Err(error) => {
                error!("error while setting AVAudioSession category: {error}");
            }
        }
    }
}
