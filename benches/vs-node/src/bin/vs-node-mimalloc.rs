//! The showcase harness with mimalloc, the allocator the djs-rs README recommends (D-080).
//! See the `djs_vs_node` crate docs.

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    djs_vs_node::main("mimalloc");
}
