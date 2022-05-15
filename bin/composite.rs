use mars_raw_utils::prelude::*;
use sciimg::{
    prelude::*,
    vector::Vector,
    min,
    max,
    quaternion::Quaternion
};

#[macro_use]
extern crate clap;
use clap::{Arg, App};
use std::process;

/// Representation of left/right side of a stereo image with an option to simply not care (or unknown).
#[derive(Debug, Copy, Clone, PartialEq)]
enum Eye {
    Right,
    Left,
    DontCare
}

/// A single two-dimensional point on a raster. Contains the x/y coordinate and the RGB values to be placed
/// into the buffer.
#[derive(Debug, Clone)]
struct Point {
    pub x:f64,
    pub y:f64,
    pub r:f64,
    pub g:f64,
    pub b:f64
}

impl Point{

    /// Simple creation for a Point.
    pub fn create(x:f64, y:f64, r:f64, g:f64, b:f64) -> Self {
        Point{
            x:x,
            y:y,
            r:r,
            g:g,
            b:b
        }
    }
}

/// A triangle (polygon) of three two-dimensional points
struct Triangle {
    pub p0:Point,
    pub p1:Point,
    pub p2:Point
}

impl Triangle {

    /// Determine if a two dimensional point is contained within the  area bounded by the triangle
    pub fn contains(&self, x:f64, y:f64) -> bool {
        let p = Point{x:x, y:y, r:0.0, g:0.0, b:0.0};
        let b0 = Triangle::sign(&p, &self.p0, &self.p1) <= 0.0;
        let b1 = Triangle::sign(&p, &self.p1, &self.p2) <= 0.0;
        let b2 = Triangle::sign(&p, &self.p2, &self.p0) <= 0.0;

        (b0 == b1) && (b1 == b2)
    }

    
    pub fn sign(p0:&Point, p1:&Point, p2:&Point) -> f64 {
        (p0.x - p2.x) * (p1.y - p2.y) - (p1.x - p2.x) * (p0.y - p2.y)
    }

    pub fn x_min(&self) -> f64 {
        min!(self.p0.x, self.p1.x, self.p2.x)
    }

    pub fn x_max(&self) -> f64 {
        max!(self.p0.x, self.p1.x, self.p2.x)
    }

    pub fn y_min(&self) -> f64 {
        min!(self.p0.y, self.p1.y, self.p2.y)
    }

    pub fn y_max(&self) -> f64 {
        max!(self.p0.y, self.p1.y, self.p2.y)
    }

    /// Determines an interpolated single-channel color value for a point in the triangle
    pub fn interpolate_color_channel(&self, x:f64, y:f64, c0:f64, c1:f64, c2:f64) -> f64 {
        let det = self.p0.x * self.p1.y - self.p1.x * self.p0.y + self.p1.x * self.p2.y - self.p2.x * self.p1.y + self.p2.x * self.p0.y - self.p0.x * self.p2.y;
        let a = ((self.p1.y-self.p2.y)*c0+(self.p2.y-self.p0.y)*c1+(self.p0.y-self.p1.y)*c2) / det;
        let b = ((self.p2.x-self.p1.x)*c0+(self.p0.x-self.p2.x)*c1+(self.p1.x-self.p0.x)*c2) / det;
        let c = ((self.p1.x*self.p2.y-self.p2.x*self.p1.y)*c0+(self.p2.x*self.p0.y-self.p0.x*self.p2.y)*c1+(self.p0.x*self.p1.y-self.p1.x*self.p0.y)*c2) / det;

        let v = a*x+b*y+c;
        v
    }

    /// Determines an interpolated three-channel (RGB) color value for a point in the triangle
    pub fn interpolate_color(&self, x:f64, y:f64) -> (f64, f64, f64) {
        let r = self.interpolate_color_channel(x, y, self.p0.r, self.p1.r, self.p2.r);
        let g = self.interpolate_color_channel(x, y, self.p0.g, self.p1.g, self.p2.g);
        let b = self.interpolate_color_channel(x, y, self.p0.b, self.p1.b, self.p2.b);
        (r, g, b)
    }

}

/// Defines a buffer that can be drawn on using triangle or square polygons.
trait Drawable {

    /// Create a simple three-channel image buffer
    fn create(width:usize, height:usize) -> Self;

    /// Paint a triangle on the buffer.
    fn paint_tri(&mut self, tri:&Triangle, avg_pixels:bool, eye:Eye);

    /// Paint a square on the buffer using four points
    fn paint_square(&mut self, tl:&Point, bl:&Point, br:&Point, tr:&Point, avg_pixels:bool, eye:Eye);

    /// Width of the buffer
    fn get_width(&self) -> usize;

    /// Height of the buffer
    fn get_height(&self) -> usize;
}

/// Implements the Drawable trait for the RgbImage class. This is probably later be merged fully into RgbImage
/// in the sciimg crate.
impl Drawable for RgbImage {
    fn create(width:usize, height:usize) -> Self {
        RgbImage::new_with_bands(width, height, 3, ImageMode::U16BIT).unwrap()
    }

    fn get_width(&self) -> usize {
        self.width
    }

    fn get_height(&self) -> usize {
        self.height
    }

    fn paint_tri(&mut self, tri:&Triangle, avg_pixels:bool, eye:Eye) {

        let min_x = tri.x_min().floor() as usize;
        let max_x = tri.x_max().ceil() as usize;
        let min_y = tri.y_min().floor() as usize;
        let max_y = tri.y_max().ceil() as usize;

        // Gonna limit the max dimension of a poly to just 100x100 
        // to prevent those that wrap the entire image. 
        // Until I plan out a better control to handle polygons that
        // wrap the cut-off azimuth
        if max_x - min_x < 100 && max_y - min_y <  100 {
            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    if x < self.width && y < self.height && tri.contains(x as f64, y as f64) {
                        let (mut r, mut g, mut b) = tri.interpolate_color(x as f64,y as f64);
                        

                        let r0 = self.get_band(0).get(x, y).unwrap() as f64;
                        let g0 = self.get_band(1).get(x, y).unwrap() as f64;
                        let b0 = self.get_band(2).get(x, y).unwrap() as f64;

                        if avg_pixels && (r0 > 0.0 || g0 > 0.0 || b0 > 0.0) {
                            r = (r + r0) / 2.0;
                            g = (g + g0) / 2.0;
                            b = (b + b0) / 2.0;
                        }

                        match eye {
                            Eye::Left => {
                                self.put(x, y, r as f32, 0);
                            },
                            Eye::Right => {
                                self.put(x, y, g as f32, 1);
                                self.put(x, y, b as f32, 2);
                            },
                            Eye::DontCare => {
                                self.put(x, y, r as f32, 0);
                                self.put(x, y, g as f32, 1);
                                self.put(x, y, b as f32, 2);
                            }
                        };
                    }
                }
            }
        }
    }

    /// Paints a square on the image by breaking it into two triangles.
    fn paint_square(&mut self, tl:&Point, bl:&Point, br:&Point, tr:&Point, avg_pixels:bool, eye:Eye) {
        self.paint_tri(&Triangle {
            p0: tl.clone(),
            p1: bl.clone(),
            p2: tr.clone()
        }, avg_pixels, eye);
        self.paint_tri(&Triangle {
            p0: tr.clone(),
            p1: bl.clone(),
            p2: br.clone()
        }, avg_pixels, eye);
    }
}


fn get_cahvor(img:&MarsImage) -> Option<CameraModel> {
    match &img.metadata {
        Some(md) => {
            if md.camera_model_component_list.is_valid() {
                Some(md.camera_model_component_list.clone())
            } else {
                None
            }
        },
        None => {
            None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct MapContext {
    pub top_lat:f64,
    pub bottom_lat:f64,
    pub left_lon:f64,
    pub right_lon:f64,
    pub width:usize,
    pub height:usize,
    pub degrees_per_pixel:f64
}

struct LatLon{
    lat:f64,
    lon:f64
}

fn intersect_to_plane(lv:&LookVector, ground:&Vector) -> Vector {
    let normal = Vector::new(0.0, 0.0, -1.0);
    

    let dot = lv.look_direction.dot_product(&normal);
    if dot == 0.0 {
        return lv.look_direction.clone();
    }

    let ratio = ground.subtract(&lv.origin).dot_product(&normal) / dot;

    let intersect_point = lv.origin.add(&lv.look_direction.scale(ratio));
    
    if ratio < 0.0 {
        lv.look_direction.clone()
    } else {
        intersect_point
    }

}

fn intersect_to_sphere(lv:&LookVector, radius:f64) -> Vector {
    lv.look_direction.normalized().scale(radius).add(&lv.origin)
}

fn vector_to_cylindrical(v:&Vector) -> LatLon {
    LatLon{
        lat:v.z.atan2((v.x * v.x + v.y * v.y).sqrt()).to_degrees(),
        lon:v.y.atan2(v.x).to_degrees() + 180.0
    }
}

fn lookvector_to_cylindrical(lv:&LookVector, quat_o:Option<&Quaternion>) -> LatLon {
    let ray = intersect_to_sphere(&lv, SPHERE_RADIUS);
    let rotated = if let Some(quat) = quat_o {
        quat.rotate_vector(&ray)
    } else {
        ray
    };
    vector_to_cylindrical(&rotated)
}

static SPHERE_RADIUS:f64 = 100.0;

fn get_lat_lon(c:&CameraModel, x:usize, y:usize) -> error::Result<LatLon> {
    match c.ls_to_look_vector(&ImageCoordinate{ line:y as f64, sample:x as f64 }) {
        Ok(lv) => {
            Ok(lookvector_to_cylindrical(&lv, None))
        },
        Err(e) => {
            Err(e)
        }
    }
}



fn determine_map_context(input_files:&Vec<&str>) -> MapContext {
    let mut context = MapContext{
        top_lat : -90.0,
        bottom_lat : 90.0,
        left_lon: 360.0,
        right_lon: -360.0,
        width: 0,
        height: 0,
        degrees_per_pixel: 0.0
    };

    input_files.iter().for_each(|input_file| {
        let img = MarsImage::open(String::from(*input_file), Instrument::M20MastcamZLeft);
        match get_cahvor(&img) {
            Some(c) => {
                match get_lat_lon(&c, 0, 0) {
                    Ok(ll) => {
                        context.bottom_lat = min!(context.bottom_lat, ll.lat );
                        context.top_lat = max!(context.top_lat, ll.lat);
                        context.left_lon = min!(context.left_lon, ll.lon);
                        context.right_lon = max!(context.right_lon, ll.lon);
                    },
                    Err(_) => {}
                };

                match get_lat_lon(&c, img.image.width, 0) {
                    Ok(ll) => {
                        context.bottom_lat = min!(context.bottom_lat, ll.lat );
                        context.top_lat = max!(context.top_lat, ll.lat);
                        context.left_lon = min!(context.left_lon, ll.lon);
                        context.right_lon = max!(context.right_lon, ll.lon);
                    },
                    Err(_) => {}
                };

                match get_lat_lon(&c, 0, img.image.height) {
                    Ok(ll) => {
                        context.bottom_lat = min!(context.bottom_lat, ll.lat );
                        context.top_lat = max!(context.top_lat, ll.lat);
                        context.left_lon = min!(context.left_lon, ll.lon);
                        context.right_lon = max!(context.right_lon, ll.lon);
                    },
                    Err(_) => {}
                };

                match get_lat_lon(&c, img.image.width, img.image.height) {
                    Ok(ll) => {
                        context.bottom_lat = min!(context.bottom_lat, ll.lat );
                        context.top_lat = max!(context.top_lat, ll.lat);
                        context.left_lon = min!(context.left_lon, ll.lon);
                        context.right_lon = max!(context.right_lon, ll.lon);
                    },
                    Err(_) => {}
                };

                match get_lat_lon(&c, img.image.width / 2, 0) {
                    Ok(ll) => {
                        context.bottom_lat = min!(context.bottom_lat, ll.lat );
                        context.top_lat = max!(context.top_lat, ll.lat);
                        context.left_lon = min!(context.left_lon, ll.lon);
                        context.right_lon = max!(context.right_lon, ll.lon);
                    },
                    Err(_) => {}
                };

                match get_lat_lon(&c, img.image.width / 2, img.image.height) {
                    Ok(ll) => {
                        context.bottom_lat = min!(context.bottom_lat, ll.lat );
                        context.top_lat = max!(context.top_lat, ll.lat);
                        context.left_lon = min!(context.left_lon, ll.lon);
                        context.right_lon = max!(context.right_lon, ll.lon);
                    },
                    Err(_) => {}
                };

                let ang_horiz = c.pixel_angle_horiz().to_degrees();
                context.degrees_per_pixel = max!(context.degrees_per_pixel, ang_horiz);

            },
            None => {}
        };

    });

    context.top_lat = min!(context.top_lat, 90.0);
    context.bottom_lat = max!(context.bottom_lat, -90.0);
    context.left_lon = max!(context.left_lon, 0.0);
    context.right_lon = min!(context.right_lon, 360.0);
    context.height = ((context.top_lat - context.bottom_lat) / context.degrees_per_pixel).floor() as usize;
    context.width = ((context.right_lon - context.left_lon) / context.degrees_per_pixel).floor() as usize;

    context
}

fn get_ls_from_map_xy(model:&CameraModel, map_context:&MapContext, x:usize, y:usize, quat:&Quaternion) -> (f64, f64) {
    let img_x = x as f64;
    let img_y = y as f64;


    let lv = match model.ls_to_look_vector(&ImageCoordinate{ line:img_y, sample: img_x }) {
        Ok(lv) => lv,
        Err(_) => panic!("Unable to convert ls to look vector")
    };

    let ll = lookvector_to_cylindrical(&lv, Some(&quat));
    let lat = ll.lat;
    let lon = ll.lon;
    
    let out_y_f = (lat - map_context.bottom_lat) / (map_context.top_lat - map_context.bottom_lat) * map_context.height as f64;
    let out_x_f = (lon - map_context.left_lon) / (map_context.right_lon - map_context.left_lon) * map_context.width as f64;

    (out_x_f, out_y_f)
}

fn process_file<D:Drawable>(input_file:&str, map_context:&MapContext, map:&mut D, anaglyph:bool, azimuth_rotation:f64) {

    let mut img = MarsImage::open(String::from(input_file), Instrument::M20MastcamZLeft);
    img.instrument = match &img.metadata {
        Some(md) => Instrument::from_str(md.instrument.as_str()),
        None => Instrument::M20MastcamZLeft
    };

    let eye = if anaglyph {
        match util::filename_char_at_pos(&input_file, 1) {
            'R' => Eye::Right,
            'L' => Eye::Left,
            _ => Eye::DontCare
        }
    } else {
        Eye::DontCare
    };

    let quat = Quaternion::from_pitch_roll_yaw(0.0, 0.0, azimuth_rotation.to_radians());


    match get_cahvor(&img) {
        Some(input_model) => {

            println!("");
            vprintln!("Input Model C: {:?}", input_model.c());
            vprintln!("Input Model A: {:?}", input_model.a());
            vprintln!("Input Model H: {:?}", input_model.h());
            vprintln!("Input Model V: {:?}", input_model.v());
            vprintln!("Input Model O: {:?}", input_model.o());
            vprintln!("Input Model R: {:?}", input_model.r());
            vprintln!("Input Model E: {:?}", input_model.e());
            println!("");

            for x in 50..(img.image.width - 51) {
                for y in 50..(img.image.height - 51) {
                    
                    let (tl_x, tl_y) = get_ls_from_map_xy(&input_model, &map_context, x, y, &quat);
                    let (tr_x, tr_y) = get_ls_from_map_xy(&input_model, &map_context, x+1, y, &quat);
                    let (bl_x, bl_y) = get_ls_from_map_xy(&input_model, &map_context, x, y+1, &quat);
                    let (br_x, br_y) = get_ls_from_map_xy(&input_model, &map_context, x+1, y+1, &quat);

                    let tl = Point::create(
                        tl_x,
                        tl_y,
                        img.image.get_band(0).get(x, y).unwrap() as f64,
                        img.image.get_band(1).get(x, y).unwrap() as f64,
                        img.image.get_band(2).get(x, y).unwrap() as f64
                    );

                    let tr = Point::create(
                        tr_x,
                        tr_y,
                        img.image.get_band(0).get(x+1, y).unwrap() as f64,
                        img.image.get_band(1).get(x+1, y).unwrap() as f64,
                        img.image.get_band(2).get(x+1, y).unwrap() as f64
                    );

                    let bl = Point::create(
                        bl_x,
                        bl_y,
                        img.image.get_band(0).get(x, y+1).unwrap() as f64,
                        img.image.get_band(1).get(x, y+1).unwrap() as f64,
                        img.image.get_band(2).get(x, y+1).unwrap() as f64
                    );

                    let br = Point::create(
                        br_x,
                        br_y,
                        img.image.get_band(0).get(x+1, y+1).unwrap() as f64,
                        img.image.get_band(1).get(x+1, y+1).unwrap() as f64,
                        img.image.get_band(2).get(x+1, y+1).unwrap() as f64
                    );


                    map.paint_square(&tl, &bl, &br, &tr, false, eye);
                }

            }

            //let output_model = input_model.linearize(img.image.width, img.image.height, img.image.width, img.image.height).unwrap();
            // vprintln!("output model: {:?}", output_model);
            // println!("");

            // let ground = Vector::new(0.0,0.0,1.84566);
            // let z = Vector::new(0.0, 0.0, -1.0);
            // let mut min_angle = 1000000.0;
            // let mut max_angle = -1000000.0;

            // for y in 0..map_context.height {
            //     for x in 0..map_context.width {

            //         if let Ok(lv) = output_model.ls_to_look_vector(&ImageCoordinate{line: y as f64, sample: x as f64}) {
                        
            //             //vprintln!("lv -> {:?}", lv.look_direction);
            //             let ray = intersect_to_plane(&lv, &ground);
            //             //vprintln!("ray -> {:?} -- {}", ray, ray.len());
            //             min_angle = min!(z.angle(&ray).to_degrees(), min_angle);
            //             max_angle = max!(z.angle(&ray).to_degrees(), max_angle);
            //             let ls_in = input_model.xyz_to_ls(&ray, false);
                        

            //             let in_x = ls_in.sample.round() as usize;
            //             let in_y = ls_in.line.round() as usize;
            //             //vprintln!("{}, {} -> Line: {}, Sample: {}", y, x, ls_in.line, ls_in.sample);


            //             if in_x < img.image.width && in_y < img.image.height {
            //                 map_r.put(x, y, img.image.get_band(0).get(in_x, in_y).unwrap());
            //                 map_g.put(x, y, img.image.get_band(1).get(in_x, in_y).unwrap());
            //                 map_b.put(x, y, img.image.get_band(2).get(in_x, in_y).unwrap());
            //             }
            //         }

            //     }
            // }
            
            // vprintln!("Min/Max angles: {}, {}", min_angle, max_angle);
            
                   
        },
        None => {
            eprintln!("CAHVOR not found for image, cannot continue");
            process::exit(2);
        }
    }


}

fn main() {
    let matches = App::new(crate_name!())
                    .version(crate_version!())
                    .author(crate_authors!())
                    .arg(Arg::with_name(constants::param::PARAM_INPUTS)
                        .short(constants::param::PARAM_INPUTS_SHORT)
                        .long(constants::param::PARAM_INPUTS)
                        .value_name("INPUT")
                        .help("Input")
                        .required(true)
                        .multiple(true)
                        .takes_value(true))
                    .arg(Arg::with_name(constants::param::PARAM_OUTPUT)
                        .short(constants::param::PARAM_OUTPUT_SHORT)
                        .long(constants::param::PARAM_OUTPUT)
                        .value_name("OUTPUT")
                        .help("Output")
                        .required(true)
                        .takes_value(true)) 
                    .arg(Arg::with_name(constants::param::PARAM_ANAGLYPH)
                        .short(constants::param::PARAM_ANAGLYPH_SHORT)
                        .long(constants::param::PARAM_ANAGLYPH)
                        .value_name("ANAGLYPH")
                        .help("Anaglyph mode")
                        .required(false)
                        .takes_value(false)) 
                    .arg(Arg::with_name(constants::param::PARAM_AZIMUTH)
                        .short(constants::param::PARAM_AZIMUTH_SHORT)
                        .long(constants::param::PARAM_AZIMUTH)
                        .value_name("PARAM_AZIMUTH")
                        .help("Azimuth rotation")
                        .required(false)
                        .takes_value(true)) 
                    .arg(Arg::with_name(constants::param::PARAM_OUTPUT)
                        .short(constants::param::PARAM_OUTPUT_SHORT)
                        .long(constants::param::PARAM_OUTPUT)
                        .value_name("OUTPUT")
                        .help("Output")
                        .required(true)
                        .takes_value(true)) 
                    .arg(Arg::with_name(constants::param::PARAM_VERBOSE)
                        .short(constants::param::PARAM_VERBOSE)
                        .help("Show verbose output"))
                    .get_matches();

    if matches.is_present(constants::param::PARAM_VERBOSE) {
        print::set_verbose(true);
    }

    let input_files: Vec<&str> = matches.values_of(constants::param::PARAM_INPUTS).unwrap().collect();
    let output = matches.value_of("output").unwrap();

    let anaglyph_mode = matches.is_present(constants::param::PARAM_ANAGLYPH);

    let azimuth_rotation = match matches.is_present(constants::param::PARAM_AZIMUTH) {
        true => {
            let s = matches.value_of(constants::param::PARAM_AZIMUTH).unwrap();
            if util::string_is_valid_f32(&s) {
                s.parse::<f64>().unwrap()
            } else {
                eprintln!("Error: Invalid number specified for blue scalar");
                process::exit(1);
            }
        },
        false => 0.0
    };

    let map_context = determine_map_context(&input_files);
    vprintln!("Map Context: {:?}", map_context);
    vprintln!("FOV Vertical: {}", map_context.top_lat - map_context.bottom_lat);
    vprintln!("FOV Horizontal: {}", map_context.right_lon - map_context.left_lon);

    if map_context.width == 0 {
        eprintln!("Output expected to have zero width. Cannot continue with that. Exiting...");
        process::exit(1);
    } else if map_context.height == 0 {
        eprintln!("Output expected to have zero height. Cannot continue with that. Exiting...");
        process::exit(1);
    }
    // let map_context = MapContext{
    //     top_lat : -90.0,
    //     bottom_lat : 90.0,
    //     left_lon: 360.0,
    //     right_lon: -360.0,
    //     width: 1024,
    //     height: 1024,
    //     degrees_per_pixel: 0.0
    // };
    // let mut map_r = ImageBuffer::new_with_fill_as_mode(map_context.width, map_context.height, 100.0, ImageMode::U16BIT).unwrap();
    // let mut map_g = ImageBuffer::new_with_fill_as_mode(map_context.width, map_context.height, 0.0, ImageMode::U16BIT).unwrap();
    // let mut map_b = ImageBuffer::new_with_fill_as_mode(map_context.width, map_context.height, 0.0, ImageMode::U16BIT).unwrap();

    let mut map = RgbImage::create(map_context.width, map_context.height);

    for in_file in input_files.iter() {
        if path::file_exists(in_file) {
            vprintln!("Processing File: {}", in_file);
            process_file(in_file, &map_context, &mut map, anaglyph_mode, azimuth_rotation);
        } else {
            eprintln!("File not found: {}", in_file);
            process::exit(1);
        }
    }

    map.normalize_to_16bit_with_max(255.0);
    map.save(output);
}