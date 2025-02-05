use crate::{
    calibrate::*, calprofile::CalProfile, enums, enums::Instrument, image::MarsImage, path, util,
    vprintln,
};

use sciimg::error;

#[derive(Copy, Clone)]
pub struct M20HeliRte {}

impl Calibration for M20HeliRte {
    fn accepts_instrument(&self, instrument: Instrument) -> bool {
        matches!(instrument, Instrument::M20HeliRte)
    }

    fn process_file(
        &self,
        input_file: &str,
        cal_context: &CalProfile,
        only_new: bool,
    ) -> error::Result<CompleteContext> {
        let out_file = util::append_file_name(input_file, cal_context.filename_suffix.as_str());
        if path::file_exists(&out_file) && only_new {
            vprintln!("Output file exists, skipping. ({})", out_file);
            return cal_warn(cal_context);
        }

        let mut raw = MarsImage::open(String::from(input_file), enums::Instrument::M20HeliRte);

        let data_max = 255.0;

        //if ! no_ilt {
        //    vprintln!("Decompanding...");
        //    raw.decompand().unwrap();
        //    data_max = decompanding::get_max_for_instrument(enums::Instrument::M20Watson) as f32;
        //}

        vprintln!("Flatfielding...");
        raw.flatfield();

        vprintln!("Applying color weights...");
        raw.apply_weight(
            cal_context.red_scalar,
            cal_context.green_scalar,
            cal_context.blue_scalar,
        );

        vprintln!("Normalizing...");
        raw.image.normalize_to_16bit_with_max(data_max);

        vprintln!("Writing to disk...");
        raw.save(&out_file);

        cal_ok(cal_context)
    }
}
