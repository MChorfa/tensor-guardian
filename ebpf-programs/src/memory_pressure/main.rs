use aya_ebpf::{macros::tracepoint, programs::TracePointContext};
use aya_log_ebpf::info;

#[tracepoint]
pub fn mm_vmscan_direct_reclaim_begin(ctx: TracePointContext) -> u32 {
    match try_vmscan_reclaim(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_vmscan_reclaim(ctx: TracePointContext) -> Result<u32, u32> {
    info!(&ctx, "tracepoint: Memory pressure - direct reclaim");
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}
