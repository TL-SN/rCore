use crate::drivers::bus::virtio::VirtioHal;
use crate::sync::UPIntrFreeCell;
use alloc::{sync::Arc, vec::Vec};
use core::any::Any;
use embedded_graphics::pixelcolor::Rgb888;
use tinybmp::Bmp;
use virtio_drivers::{VirtIOGpu, VirtIOHeader};
const VIRTIO7: usize = 0x10007000;      // 这是 Qemu模拟的virtio_gpu设备中I/O寄存器的物理内存地址， VirtIOGpu 需要这个地址来对 VirtIOHeader 数据结构所表示的virtio-gpu I/O控制寄存器进行读写操作，从而完成对某个具体的virtio-gpu设备的初始化过程。
pub trait GpuDevice: Send + Sync + Any {
    fn update_cursor(&self);                    // //更新光标，目前暂时没用
    fn get_framebuffer(&self) -> &mut [u8];
    fn flush(&self);
}

lazy_static::lazy_static!(
    pub static ref GPU_DEVICE: Arc<dyn GpuDevice> = Arc::new(VirtIOGpuWrapper::new());
);

pub struct VirtIOGpuWrapper {
    gpu: UPIntrFreeCell<VirtIOGpu<'static, VirtioHal>>,
    fb: &'static [u8],              // 一维字节数组引用表示的显存缓冲区 
}
static BMP_DATA: &[u8] = include_bytes!("../../assert/mouse.bmp");
impl VirtIOGpuWrapper {
    pub fn new() -> Self {
        unsafe {
              // 1. 执行virtio-drivers的gpu.rs中virto-gpu基本初始化
            let mut virtio =
                VirtIOGpu::<VirtioHal>::new(&mut *(VIRTIO7 as *mut VirtIOHeader)).unwrap();

             // 2. 设置virtio-gpu设备的显存，初始化显存的一维字节数组引用
            let fbuffer = virtio.setup_framebuffer().unwrap();
            let len = fbuffer.len();
            let ptr = fbuffer.as_mut_ptr();
            let fb = core::slice::from_raw_parts_mut(ptr, len);

            // 3. 初始化光标图像的像素值
            let bmp = Bmp::<Rgb888>::from_slice(BMP_DATA).unwrap();
            let raw = bmp.as_raw();
            let mut b = Vec::new();
            for i in raw.image_data().chunks(3) {
                let mut v = i.to_vec();
                b.append(&mut v);
                if i == [255, 255, 255] {
                    b.push(0x0)
                } else {
                    b.push(0xff)
                }
            }
            // 4. 设置virtio-gpu设备的光标图像
            virtio.setup_cursor(b.as_slice(), 50, 50, 50, 50).unwrap();
        // 5. 返回VirtIOGpuWrapper结构类型
            Self {
                gpu: UPIntrFreeCell::new(virtio),
                fb,
            }
        }
    }
}

impl GpuDevice for VirtIOGpuWrapper {
    // // 通知virtio-gpu设备更新图形显示内容
    fn flush(&self) {
        self.gpu.exclusive_access().flush().unwrap();
    }
      // 得到显存的基于内核态虚地址的一维字节数组引用
    fn get_framebuffer(&self) -> &mut [u8] {
        unsafe {
            let ptr = self.fb.as_ptr() as *const _ as *mut u8;
            core::slice::from_raw_parts_mut(ptr, self.fb.len())
        }
    }
    fn update_cursor(&self) {}
}
