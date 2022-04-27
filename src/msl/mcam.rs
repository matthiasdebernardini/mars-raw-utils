use crate::{
    vprintln, 
    image::MarsImage, 
    enums, 
    path,
    util,
    decompanding,
    flatfield,
    inpaintmask,
    calprofile,
    print::{
        print_done,
        print_fail,
        print_warn
    }
};

use sciimg::{
    enums::ImageMode
};



pub fn process_with_profiles(input_file:&str, red_scalar:f32, green_scalar:f32, blue_scalar:f32, color_noise_reduction:i32, no_ilt:bool, only_new:bool, filename_suffix:&String, profile_names:&Vec<&str>) {

    if profile_names.len() > 0 {
        for f in profile_names.iter() {
            process_with_profile(input_file, red_scalar, green_scalar, blue_scalar, color_noise_reduction, no_ilt, only_new, filename_suffix, Some(&f.to_string()));
        }
    } else {
        process_with_profile(input_file, red_scalar, green_scalar, blue_scalar, color_noise_reduction, no_ilt, only_new, filename_suffix, None);
    }

}

pub fn process_with_profile(input_file:&str, red_scalar:f32, green_scalar:f32, blue_scalar:f32, color_noise_reduction:i32, no_ilt:bool, only_new:bool, filename_suffix:&String, profile_name_opt:Option<&String>) {

    if let Some(profile_name) = profile_name_opt {

        match calprofile::load_calibration_profile(&profile_name.to_string()) {
            Ok(profile) => {
                process_file(input_file, profile.red_scalar, profile.green_scalar, profile.blue_scalar, profile.color_noise_reduction_amount, !profile.apply_ilt, only_new, &profile.filename_suffix);
            },
            Err(why) => {
                eprintln!("Error loading calibration profile: {}", why);
                print_fail(&format!("{} ({})", path::basename(input_file), filename_suffix));
                panic!("Error loading calibration profile");
            }
        }
    } else {
        process_file(input_file, red_scalar, green_scalar, blue_scalar, color_noise_reduction, no_ilt, only_new, &filename_suffix);
    }

}

fn process_file(input_file:&str, red_scalar:f32, green_scalar:f32, blue_scalar:f32, color_noise_reduction:i32, no_ilt:bool, only_new:bool, filename_suffix:&String) {
    let out_file = util::append_file_name(input_file, filename_suffix);
    if path::file_exists(&out_file) && only_new {
        vprintln!("Output file exists, skipping. ({})", out_file);
        print_warn(&format!("{} ({})", path::basename(input_file), filename_suffix));
        return;
    }

    let mut instrument = enums::Instrument::MslMastcamLeft;

    if util::filename_char_at_pos(&input_file, 5) == 'R' {
        instrument = enums::Instrument::MslMastcamRight;
        vprintln!("Processing for Mastcam Right");
    } else {
        vprintln!("Processing for Mastcam Left") ;
    }

    let mut raw = MarsImage::open(String::from(input_file), instrument);

    let mut data_max = 255.0;
    
    if ! no_ilt {
        vprintln!("Decompanding...");
        raw.decompand(&decompanding::get_ilt_for_instrument(instrument));
        data_max = decompanding::get_max_for_instrument(instrument) as f32;
    }

    if /*util::filename_char_at_pos(&input_file, 22) == 'E' &&*/ raw.image.is_grayscale() {
        vprintln!("Image appears to be grayscale, applying debayering...");
        raw.debayer();
    }


    let mut inpaint_mask = inpaintmask::load_mask(instrument).unwrap();
    let mut flat = flatfield::load_flat(instrument).unwrap();


    if raw.image.width == 1536 {
        raw.image.crop(161, 0, 1328, raw.image.height);
    }

    if raw.image.height == 1600 && raw.image.height == 1200 {
        raw.image.crop(125, 13, 1328, 1184);
    }


    
    vprintln!("Flatfielding...");
    

    if instrument == enums::Instrument::MslMastcamRight {

        if raw.image.width == 1328 && raw.image.height == 1184 {
            //x160, y16
            flat.image.crop(160, 16, 1328, 1184);
            inpaint_mask = inpaint_mask.get_subframe(160, 16, 1328, 1184).unwrap();
        } else if raw.image.width == 848 && raw.image.height == 848 {
            //x400, y192
            flat.image.crop(400, 192, 848, 848);
            inpaint_mask = inpaint_mask.get_subframe(400, 192, 848, 848).unwrap();
        } else if raw.image.width == 1344 && raw.image.height == 1200 {
            //x400, y192
            flat.image.crop(160, 0, 1344, 1200);
            inpaint_mask = inpaint_mask.get_subframe(160, 0, 1344, 1200).unwrap();
        }

        if raw.image.get_mode() == ImageMode::U8BIT {
            flat.image.normalize_to_12bit_with_max(decompanding::get_max_for_instrument(instrument) as f32, 255.0);
            flat.compand(&decompanding::get_ilt_for_instrument(instrument));
        }

    }

    if instrument == enums::Instrument::MslMastcamLeft {

        if raw.image.width == 1328 && raw.image.height == 1184 { //9
            flat.image.crop(160, 16, 1328, 1184);
            inpaint_mask = inpaint_mask.get_subframe(160, 16, 1328, 1184).unwrap();
        }  else if raw.image.width == 1152 && raw.image.height == 432 {
            flat.image.crop(305, 385, 1152, 432);
            inpaint_mask = inpaint_mask.get_subframe(305, 385, 1152, 432).unwrap();
        } else if raw.image.width == 1600 && raw.image.height == 1200 {
            flat.image.crop(33, 0, 1600, 1200);
            inpaint_mask = inpaint_mask.get_subframe(33, 0, 1600, 1200).unwrap();
        } else if raw.image.width == 1456 && raw.image.height == 640 {
            flat.image.crop(96, 280, 1456, 640);
            inpaint_mask = inpaint_mask.get_subframe(96, 280, 1456, 640).unwrap();
        }

        if raw.image.get_mode() == ImageMode::U8BIT {
            flat.image.normalize_to_12bit_with_max(decompanding::get_max_for_instrument(instrument) as f32, 255.0);
            flat.compand(&decompanding::get_ilt_for_instrument(instrument));
        }
    }

    vprintln!("Raw: {}/{}, Flat: {}/{}", raw.image.width, raw.image.height, flat.image.width, flat.image.height);

    // Catch some subframing edge cases
    if flat.image.width > raw.image.width {
        let x = (flat.image.width - raw.image.width) / 2;
        let y = (flat.image.height - raw.image.height) / 2;
        vprintln!("Cropping flat/inpaint mask with x/y/width/height: {},{} {}x{}", x, y, raw.image.width, raw.image.height);
        flat.image.crop(x, y, raw.image.width, raw.image.height);
        inpaint_mask = inpaint_mask.get_subframe(x, y, raw.image.width, raw.image.height).unwrap();
    }

    flat.apply_inpaint_fix_with_mask(&inpaint_mask);

    vprintln!("Raw: {}/{}, Flat: {}/{}", raw.image.width, raw.image.height, flat.image.width, flat.image.height);

    raw.flatfield_with_flat(&flat);

    // Only inpaint with the same size as the mask until we can reliably determine
    // subframing sensor location.
    //if raw.image.width == 1328 && raw.image.height == 1184 {
        vprintln!("Inpainting...");
        raw.apply_inpaint_fix_with_mask(&inpaint_mask);
    //}

    vprintln!("Applying color weights...");
    raw.apply_weight(red_scalar, green_scalar, blue_scalar);

    if color_noise_reduction > 0 {
        vprintln!("Color noise reduction...");
        raw.image.reduce_color_noise(color_noise_reduction);
    }
    
    vprintln!("Normalizing...");
    raw.image.normalize_to_16bit_with_max(data_max);

    vprintln!("Cropping...");
    raw.image.crop(3, 3, raw.image.width - 6, raw.image.height - 6);


    vprintln!("Writing to disk...");
    raw.save(&out_file);

    print_done(&format!("{} ({})", path::basename(input_file), filename_suffix));
    
}

