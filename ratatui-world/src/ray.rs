use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};
use lib::plane::{Plane, partition_index};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct Ray {
    world_vector: Vec3,
    screen_vector: Vec2,
}

impl Ray {
    pub fn new(world_vector: Vec3, screen_vector: Vec2) -> Self {
        Self {
            world_vector,
            screen_vector,
        }
    }

    pub fn world_vector(&self) -> Vec3 {
        self.world_vector
    }

    pub fn screen_vector(&self) -> Vec2 {
        self.screen_vector
    }

    pub fn partition_index(&self, planes: &[Plane]) -> Option<usize> {
        partition_index(planes, self.world_vector())
    }
}
