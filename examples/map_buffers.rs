extern crate ocl;
extern crate ocl_core;

use std::mem;

const BUFFER_DIMENSIONS: usize = 2 << 20;
const PLATFORM_ID: usize = 0;
const DEVICE_ID: usize = 0;

fn scalar_map() {
    let kernel_src = r#"
        __kernel void add (__global float* in, float scalar) {
            in[get_global_id(0)] += scalar;
        }
        "#;

    let plt = ocl::Platform::list()[PLATFORM_ID];
    let dev = ocl::Device::list_all(&plt).unwrap()[DEVICE_ID];
    let context = ocl::Context::builder()
        .platform(plt)
        .devices(dev)
        .build()
        .unwrap();
    let queue = ocl::Queue::new(&context, dev).unwrap();
    let program = ocl::Program::builder()
        .src(kernel_src)
        .devices(dev)
        .build(&context)
        .unwrap();

    // Creation of buffer using ocl API will result in filling of the buffer as well
    let in_buff = ocl::Buffer::new(queue.clone(),
                                   Some(ocl::core::MEM_ALLOC_HOST_PTR),
                                   &[BUFFER_DIMENSIONS],
                                   None::<&[f32]>)
        .expect("Creating buffer failed");

    unsafe {
        let buff_datum = ocl_core::enqueue_map_buffer::<f32>(&queue,
                                                             in_buff.core_as_ref(),
                                                             true,
                                                             ocl_core::MAP_WRITE,
                                                             0,
                                                             BUFFER_DIMENSIONS,
                                                             None,
                                                             None)
            .expect("Mapping memory object failed");
        // Wait until mapping is finished
        queue.finish();
        let mut buff_vector =
            Vec::from_raw_parts(buff_datum as *mut f32, BUFFER_DIMENSIONS, BUFFER_DIMENSIONS);

        let datum: Vec<f32> = vec![10_f32; BUFFER_DIMENSIONS];
        buff_vector.copy_from_slice(&datum);

        ocl_core::enqueue_unmap_mem_object(&queue, in_buff.core_as_ref(), buff_datum, None, None)
            .expect("Unmap of memory object failed");
        // Wait until unmapping is finished
        queue.finish();
        // Don't deallocate vector, it'll lead to double free of the buffer pointed by buff_datum
        mem::forget(buff_vector);
    }
    let mut check_datum: Vec<f32> = vec![0_f32; BUFFER_DIMENSIONS];
    in_buff.read(&mut check_datum)
        .enq()
        .expect("Reading from in_buff failed");
    for &ele in check_datum.iter() {
        assert_eq!(ele, 10_f32);
    }

    ocl::Kernel::new(String::from("add"), &program, &queue)
        .expect("Kernel creation failed")
        .gws([BUFFER_DIMENSIONS])
        .arg_buf(&in_buff)
        .arg_scl(5_f32)
        .cmd()
        .enq()
        .expect("Kernel execution failed");

    let mut read_datum: Vec<f32> = vec![0_f32; BUFFER_DIMENSIONS];
    in_buff.read(&mut read_datum)
        .enq()
        .expect("Reading from in_buff after kernel exec failed");
    for &ele in read_datum.iter() {
        assert_eq!(ele, 15_f32);
    }
}

fn vector_map() {
    let kernel_src = r#"
        __kernel void add (__global float16* in, float scalar) {
            float16 invalue = in[get_global_id(0)];
            /* Use only first value */
            invalue.s0 += scalar;
            in[get_global_id(0)] = invalue;
        }
        "#;
    let plt = ocl::Platform::list()[PLATFORM_ID];
    let dev = ocl::Device::list_all(&plt).unwrap()[DEVICE_ID];
    let context = ocl::Context::builder()
        .platform(plt)
        .devices(dev)
        .build()
        .unwrap();
    let queue = ocl::Queue::new(&context, dev).unwrap();
    let program = ocl::Program::builder()
        .src(kernel_src)
        .devices(dev)
        .build(&context)
        .unwrap();
    let in_buff = ocl::Buffer::new(queue.clone(),
                                   Some(ocl::core::MEM_ALLOC_HOST_PTR),
                                   &[BUFFER_DIMENSIONS],
                                   None::<&[ocl::aliases::ClFloat16]>)
        .expect("Creating buffer failed");

    unsafe {
        let mut event = ocl::EventList::new();
        let buff_datum =
            ocl_core::enqueue_map_buffer::<ocl::aliases::ClFloat16>(&queue,
                                                                    in_buff.core_as_ref(),
                                                                    true,
                                                                    ocl_core::MAP_WRITE,
                                                                    0,
                                                                    BUFFER_DIMENSIONS,
                                                                    None,
                                                                    Some(&mut event))
                .expect("Mapping memory object failed");
        queue.finish();
        let mut buff_vector = Vec::from_raw_parts(buff_datum as *mut ocl::aliases::ClFloat16,
                                                  BUFFER_DIMENSIONS,
                                                  BUFFER_DIMENSIONS);

        let mut value: ocl::aliases::ClFloat16 = Default::default();
        // Use only first value
        value.0 = 10_f32;
        let datum: Vec<ocl::aliases::ClFloat16> = vec![value; BUFFER_DIMENSIONS];
        buff_vector.copy_from_slice(&datum);
        ocl_core::enqueue_unmap_mem_object(&queue, in_buff.core_as_ref(), buff_datum, None, None)
            .expect("Unmap of memory object failed");
        queue.finish();
        mem::forget(buff_vector);
    }
    let mut check_datum: Vec<ocl::aliases::ClFloat16> = vec![Default::default(); BUFFER_DIMENSIONS];

    in_buff.read(&mut check_datum)
        .enq()
        .expect("Reading from in_buff failed");

    for &ele in check_datum.iter() {
        assert_eq!(ele.0, 10_f32);
    }

    ocl::Kernel::new(String::from("add"), &program, &queue)
        .expect("Kernel creation failed")
        .gws([BUFFER_DIMENSIONS])
        .arg_buf(&in_buff)
        .arg_scl(5_f32)
        .cmd()
        .enq()
        .expect("Kernel execution failed");

    let mut read_datum: Vec<ocl::aliases::ClFloat16> = vec![Default::default(); BUFFER_DIMENSIONS];
    in_buff.read(&mut read_datum)
        .enq()
        .expect("Reading from in_buff after kernel exec failed");

    for &ele in read_datum.iter() {
        assert_eq!(ele.0, 15_f32);
    }
}

fn main() {
    scalar_map();
    vector_map();
}
