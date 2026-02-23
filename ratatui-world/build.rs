use spirv_builder::SpirvBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder =  SpirvBuilder::new("../shader", "spirv-unknown-spv1.6");

    builder.build_script.defaults = true;
    builder.build_script.env_shader_spv_path = Some(true);

    builder
        .build().unwrap();

    Ok(())
}
