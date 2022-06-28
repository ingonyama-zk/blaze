use num::traits::Pow;
pub fn num_of_elements_oct_tree(tree_height: u32) -> u32 {
    let mut sum = 0;

    for i in 0..tree_height {
        sum += Pow::pow(8, tree_height - i - 1) as u32;
    }

    sum
}

pub fn num_of_elements_in_base_layer(tree_height: u32) -> u32 {
    Pow::pow(8, tree_height - 1) as u32
}

#[repr(u8)]
#[derive(PartialEq, Eq, Copy, Clone)]
pub enum TreeMode {
    TreeC,
    TreeD,
}

impl TreeMode {
    pub fn value(tree_mode: TreeMode) -> u32 {
        match tree_mode {
            TreeMode::TreeC => 0,
            TreeMode::TreeD => 1,
        }
    }
}
