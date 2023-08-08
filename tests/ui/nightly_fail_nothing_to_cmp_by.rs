use cmp_by_derive::CmpBy;

#[derive(CmpBy)]
struct Thing {
    a: u32,
    b: u64,
    f: f64,
}

fn main() {}
