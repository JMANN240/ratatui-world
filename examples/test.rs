use glam::dvec2;

fn main() {
    let character_x_resolution = 2; // how many subpixels there are along the width of a character
    let character_y_resolution = 4; // how many subpixels there are along the height of a character

    // lets say that the canvas is 60 characters wide and 60 characters tall

    let resolution_x = 60 * character_x_resolution; // 60 * 2 = 120 for quadrant
    let resolution_y = 60 * character_y_resolution; // 60 * 2 = 120 for quadrant

    // so we really have 120 pixels to work with both ways with quadrant

    let cell_aspect_ratio = 1.0 / 2.0;

    // but the characters are twice as tall as they are width, meaning that the whole "square" canvas is actually a rectangle

    let aspect_ratio =
        cell_aspect_ratio * character_y_resolution as f64 / character_x_resolution as f64;

    // So we get the aspect ratio of each pixel

    // 0.5 * 1 / 1 = 0.5 in the case of a block
    // 0.5 * 2 / 2 = 0.5 in the case of a quadrant
    // 0.5 * 4 / 2 = 1.0 in the case of braille

    let width = (resolution_x as f64); // The space on the screen is as wide as there are pixels. 120 for quadrant
    let height = (resolution_y as f64) / aspect_ratio; // The space on the screen is as tall as there are pixels divided by the AR. 120 / 0.5 = 240 for quadrant

    let cell_spacing_x = width / (60 as f64); // For quadrant it is 120 / 60 = 2, so each cell should be 2 apart width-wise
    let cell_spacing_y = height / (60 as f64); // For quadrant it is 240 / 60 = 4, so each cell should be 4 apart height-wise

    let pixel_spacing_x = cell_spacing_x / (character_x_resolution as f64); // For quadrant it is 2 / 2 = 1, so each pixel should be 1 apart width-wise
    let pixel_spacing_y = cell_spacing_y / (character_y_resolution as f64); // For quadrant it is 4 / 2 = 2, so each pixel should be 2 apart height-wise

    let left = -width / 2.0; // -120 / 2 = -60
    let right = width / 2.0; // 120 / 2 = 60
    let up = height / 2.0; // 240 / 2 = 120
    let down = -height / 2.0; // -240 / 2 = -120

    let cells = (0..60)
        .flat_map(|cell_x_index| {
            let ray_x_base = left + cell_x_index as f64 * cell_spacing_x + pixel_spacing_x / 2.0;

            // -60 + 0 * 2 + 0.5 = -59.5
            // -60 + 1 * 2 + 0.5 = -57.5
            // -60 + 2 * 2 + 0.5 = -55.5

            (0..60).map(move |cell_y_index| {
                let ray_y_base =
                    down + cell_y_index as f64 * cell_spacing_y + pixel_spacing_y / 2.0;

                // -120 + 0 * 4 + 1 = -119
                // -120 + 1 * 4 + 1 = -115
                // -120 + 2 * 4 + 1 = -111

                // eprintln!("{ray_y_base}");

                for ray_x_index in 0..character_x_resolution {
                    let ray_x_offset = ray_x_index as f64 * pixel_spacing_x;

                    for ray_y_index in 0..character_y_resolution {
                        let ray_y_offset = ray_y_index as f64 * pixel_spacing_y;

                        println!("{:?}",
                                dvec2(
                                    ray_x_base + ray_x_offset as f64,
                                    ray_y_base + ray_y_offset as f64,
                                ),
                        );
                    }
                }
            })
        })
        .collect::<Vec<()>>();
}
