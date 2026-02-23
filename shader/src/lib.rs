#![cfg_attr(target_arch = "spirv", no_std)]

use glam::{UVec3, Vec3, Vec3A};
use lib::{moller_trumbore_intersection, triangle::Triangle};
use spirv_std::{glam, spirv};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Params {
    pub width: u32,
    pub height: u32,
}

#[spirv(compute(threads(16, 16)))]
pub fn main_cs(
    #[spirv(global_invocation_id)] id: UVec3,
    #[spirv(uniform, descriptor_set = 0, binding = 0)] params: &Params,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 1)] rays: &[Vec3A],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 2)] triangles: &[Triangle],
    #[spirv(storage_buffer, descriptor_set = 0, binding = 3)] output: &mut [f32],
) {
    if id.x >= params.width || id.y >= params.height {
        return;
    }

    let camera_position = Vec3::ZERO;

    let index = (id.x + id.y * params.width) as usize;

    let ray = rays[index];

    output[index] = ray.length();
}
