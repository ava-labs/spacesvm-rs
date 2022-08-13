#[macro_export]
macro_rules! expr {
    ($e: expr) => {
        $e
    };
}
#[macro_export]
macro_rules! defer {
    ($($data: tt)*) => (
        let _scope_call = crate::macros::Scope {
            c: Some(|| -> () { expr!({ $($data)* }) })
        };
    )
}
pub struct Scope<F: FnOnce()> {
    c: Option<F>,
}
impl<F: FnOnce()> Drop for Scope<F> {
    fn drop(&mut self) {
        self.c.take().unwrap()()
    }
}
