use aya_ebpf::{macros::tracepoint, programs::TracePointContext};
use aya_log_ebpf::info;

#[tracepoint]
pub fn drm_vblank_event(ctx: TracePointContext) -> u32 {
    match try_drm_vblank_event(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_drm_vblank_event(ctx: TracePointContext) -> Result<u32, u32> {
    info!(&ctx, "tracepoint: DRM vblank event");
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
