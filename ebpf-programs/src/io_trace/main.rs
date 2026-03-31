use aya_ebpf::{macros::kprobe, programs::ProbeContext};
use aya_log_ebpf::info;

#[kprobe]
pub fn dma_map_page(ctx: ProbeContext) -> u32 {
    match try_dma_map_page(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_dma_map_page(ctx: ProbeContext) -> Result<u32, u32> {
    info!(&ctx, "kprobe: DMA map page");
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
