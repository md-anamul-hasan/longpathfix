//! Benchmark runner for comparing LongPathFix 2.0.0 Rust engine against the 1.1.0 .NET baseline.

use longpathfix_core::planner::{Planner, CLASSIC_LIMIT};
use longpathfix_core::rules::FixKinds;
use longpathfix_core::scanner::scan_with_progress;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
use windows_sys::Win32::System::Threading::GetCurrentProcess;

fn peak_working_set_mb() -> u64 {
    let mut pmc: PROCESS_MEMORY_COUNTERS = unsafe { std::mem::zeroed() };
    pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    // SAFETY: GetCurrentProcess() returns a pseudo-handle; `pmc` is properly sized.
    let ok = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb) };
    if ok != 0 {
        (pmc.PeakWorkingSetSize / (1024 * 1024)) as u64
    } else {
        0
    }
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .expect("Usage: cargo run --release -p longpathfix-core --example bench -- <path>");
    let cancel = AtomicBool::new(false);

    for run in 0..3 {
        let t0 = Instant::now();
        let scan_out = scan_with_progress(&root, &cancel, |_| {}).expect("scan failed");
        let scan_ms = t0.elapsed().as_millis();
        let t1 = Instant::now();
        let plan = Planner::build(&root, &scan_out.nodes, CLASSIC_LIMIT, FixKinds::ALL);
        let plan_ms = t1.elapsed().as_millis();
        let total_ms = t0.elapsed().as_millis();

        println!(
            "run {run}: entries={} scan_ms={scan_ms} plan_ms={plan_ms} total_ms={total_ms} findings={} ops={} peak_ws_mb={}",
            scan_out.nodes.len(),
            plan.findings.len(),
            plan.ops.len(),
            peak_working_set_mb()
        );
    }
}
