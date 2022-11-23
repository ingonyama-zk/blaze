use rust_rw_device_g2::{
    input_generator_biguint, result_check_biguint, rw_msm_to_dram::msm_calc_biguint,
};

fn main() {
    //TODO: move this to lib.rs test
    println!("Generating MSM input ...");
    // init();

    let size = 2048;
    let (points, scalars, msm_result, _, _) = input_generator_biguint(size);
    let result = msm_calc_biguint(&points, &scalars, size);
    result_check_biguint(result.0, msm_result);
}
